//! Almacenamiento local protegido de este dispositivo: hoy, la clave
//! privada del firmante de escritorio (ver `nube::firmante`).
//!
//! - **Escritorio** (feature `cifrado-secreto-dispositivo`): protegido con
//!   DPAPI de Windows (`CryptProtectData`/`CryptUnprotectData`, scope del
//!   usuario actual). DPAPI liga el blob al material que gestiona Windows
//!   para esa cuenta, no a un identificador público que cualquiera con
//!   acceso a la máquina pueda recalcular. Ver `mod dpapi` más abajo.
//! - **Sin el feature** (tests, Linux): texto plano, mismo criterio que el
//!   resto de la base local en esos entornos.
//! - **Android** no pasa por acá: su clave vive en Android Keystore y nunca
//!   sale del hardware (ver `AlmacenClaveKeystore.kt`).

use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// Cifrado real en escritorio: DPAPI de Windows, no una clave
/// derivada de un identificador público. Ver el doc-comment del módulo.
#[cfg(all(windows, feature = "cifrado-secreto-dispositivo"))]
mod dpapi {
    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::Security::Cryptography::{
        CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
    };

    /// Antepuesto al blob protegido -- distingue esto de un archivo en texto
    /// plano sin necesitar intento-y-error.
    const MAGIC: &[u8; 4] = b"DPA1";

    pub(super) fn es_protegido(contenido: &[u8]) -> bool {
        contenido.starts_with(MAGIC)
    }

    /// `None` sólo si `CryptProtectData` en sí falla -- quien llama cae a
    /// texto plano antes que perder el dato.
    pub(super) fn proteger(texto: &str) -> Option<Vec<u8>> {
        let blob = proteger_bytes(texto.as_bytes())?;
        let mut salida = Vec::with_capacity(MAGIC.len() + blob.len());
        salida.extend_from_slice(MAGIC);
        salida.extend_from_slice(&blob);
        Some(salida)
    }

    /// `None` si el archivo está corrupto o quedó de otro usuario/máquina --
    /// DPAPI simplemente no puede desprotegerlo, sin distinción posible ni
    /// falta que hace.
    pub(super) fn desproteger(contenido: &[u8]) -> Option<String> {
        let blob = contenido.strip_prefix(MAGIC)?;
        let plano = desproteger_bytes(blob)?;
        String::from_utf8(plano).ok()
    }

    /// Nunca `CRYPTPROTECT_LOCAL_MACHINE`: ese flag dejaría que cualquier
    /// usuario de este mismo Windows desprotegiera el blob. Con el scope por
    /// defecto (usuario actual), sólo esta cuenta puede recuperarlo.
    fn proteger_bytes(datos: &[u8]) -> Option<Vec<u8>> {
        let mut entrada = CRYPT_INTEGER_BLOB {
            cbData: u32::try_from(datos.len()).ok()?,
            pbData: datos.as_ptr().cast_mut(),
        };
        let mut salida = CRYPT_INTEGER_BLOB::default();

        // SAFETY: `entrada` apunta a `datos`, vivo durante toda la llamada;
        // el buffer de `salida` lo reserva la propia API y se libera con
        // LocalFree apenas se copia a un `Vec` nuestro.
        let resultado = unsafe {
            CryptProtectData(
                std::ptr::addr_of_mut!(entrada),
                None,
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                std::ptr::addr_of_mut!(salida),
            )
        };
        resultado.ok()?;
        Some(copiar_y_liberar(salida))
    }

    fn desproteger_bytes(blob: &[u8]) -> Option<Vec<u8>> {
        let mut entrada = CRYPT_INTEGER_BLOB {
            cbData: u32::try_from(blob.len()).ok()?,
            pbData: blob.as_ptr().cast_mut(),
        };
        let mut salida = CRYPT_INTEGER_BLOB::default();

        // SAFETY: mismo criterio que en `proteger_bytes`;
        // `CRYPTPROTECT_UI_FORBIDDEN` evita que un blob corrupto/ajeno
        // dispare un diálogo nativo de Windows.
        let resultado = unsafe {
            CryptUnprotectData(
                std::ptr::addr_of_mut!(entrada),
                None,
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                std::ptr::addr_of_mut!(salida),
            )
        };
        resultado.ok()?;
        Some(copiar_y_liberar(salida))
    }

    fn copiar_y_liberar(blob: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        if blob.pbData.is_null() || blob.cbData == 0 {
            return Vec::new();
        }
        // SAFETY: `blob.pbData` apunta a `blob.cbData` bytes válidos,
        // escritos por CryptProtectData/CryptUnprotectData justo antes.
        let copia =
            unsafe { std::slice::from_raw_parts(blob.pbData, blob.cbData as usize) }.to_vec();
        // SAFETY: `blob.pbData` la reservó DPAPI con LocalAlloc -- LocalFree
        // es la contraparte documentada para liberarla.
        unsafe {
            let _ = LocalFree(Some(HLOCAL(blob.pbData.cast::<std::ffi::c_void>())));
        }
        copia
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn proteger_y_desproteger_devuelve_el_mismo_secreto() {
            let protegido = proteger("s3cr3t0-dpapi").unwrap();
            assert!(es_protegido(&protegido));
            assert_eq!(desproteger(&protegido).as_deref(), Some("s3cr3t0-dpapi"));
        }

        #[test]
        fn el_blob_protegido_no_contiene_el_secreto_en_claro() {
            let protegido = proteger("s3cr3t0-visible-si-esto-fallara").unwrap();
            assert!(
                !protegido
                    .windows(b"s3cr3t0-visible-si-esto-fallara".len())
                    .any(|v| v == b"s3cr3t0-visible-si-esto-fallara")
            );
        }

        #[test]
        fn un_blob_corrupto_no_desprotege() {
            assert_eq!(desproteger(b"DPA1basura-no-es-un-blob-dpapi-valido"), None);
        }
    }
}

/// Variable de entorno del perfil "roaming" de Windows -- deliberadamente
/// DISTINTA de `LOCAL_APP_DATA_ENV` (donde vive `control_acceso.db`, ver
/// `database::connection`). Hasta 2026-09-18 la credencial vivía en la misma
/// carpeta que la base: un evento que corrompe/borra esa carpeta (disco,
/// reinstalación que hace `rd /s` sobre ella) se llevaba puesto también lo
/// que autentica a este dispositivo -- la "recuperación" dejaba de
/// ser "reconstruir desde la nube" y pasaba a ser "este dispositivo ya no
/// puede ni pedir sus propios datos" (ver `docs/recuperacion-sitio-local.md`).
/// `%APPDATA%` es un árbol de directorios distinto de `%LOCALAPPDATA%` --
/// borrar/corromper la carpeta local de la app no lo toca. Mismo criterio
/// aplicado a `db_key.dat` en `desktop/src-tauri/src/clave_cifrado.rs`
/// (misma carpeta, mismo motivo).
pub const ROAMING_APP_DATA_ENV: &str = "APPDATA";

/// Público para que `desktop/src-tauri` pueda resolver la MISMA carpeta al
/// guardar `db_key.dat` -- ambos secretos separados de la base, pero juntos
/// entre sí no reintroduce el problema: lo que importa es no compartir
/// carpeta con `control_acceso.db`, no aislarse mutuamente (ninguno de los
/// dos expone nada si se copian juntos -- DPAPI liga cada uno al usuario de
/// Windows que lo creó).
#[must_use]
pub fn directorio_credenciales_roaming() -> Option<PathBuf> {
    let root = std::env::var_os(ROAMING_APP_DATA_ENV)?;
    let root = PathBuf::from(root);
    if !root.is_absolute() {
        return None;
    }
    Some(root.join("ControlAcceso"))
}

/// Machine GUID de Windows -- ya NO se usa para derivar ninguna clave de
/// cifrado (ver el doc-comment del módulo, DPAPI no lo necesita). Queda
/// exclusivamente para la metadata forense de la activación inicial --
/// `desktop-tauri` la usa ahí, ver `comandos::nube::metadata_de_esta_maquina`.
/// `None` si no se pudo leer (registro inaccesible, permisos).
#[cfg(feature = "cifrado-secreto-dispositivo")]
pub fn identificador_de_esta_maquina() -> Option<String> {
    machine_uid::get().ok()
}

/// Escribe `texto` en `<directorio>/<archivo>`: DPAPI en escritorio con
/// `cifrado-secreto-dispositivo`, texto plano en cualquier otro caso. Lo usa
/// la clave privada del firmante de escritorio (ver `nube::firmante`). La escritura
/// pasa por un archivo temporal y un `rename`, para que un corte a mitad de
/// camino nunca deje el archivo a medio escribir.
pub(crate) fn guardar_protegido_en(
    directorio: &Path,
    archivo: &str,
    texto: &str,
) -> io::Result<()> {
    fs::create_dir_all(directorio)?;
    #[cfg(all(windows, feature = "cifrado-secreto-dispositivo"))]
    let contenido = dpapi::proteger(texto).unwrap_or_else(|| texto.as_bytes().to_vec());
    #[cfg(not(all(windows, feature = "cifrado-secreto-dispositivo")))]
    let contenido = texto.as_bytes().to_vec();

    let destino = directorio.join(archivo);
    let temporal = directorio.join(format!("{archivo}.tmp"));
    fs::write(&temporal, contenido)?;
    fs::rename(&temporal, &destino)
}

/// Ver [`guardar_protegido_en`]. `None` si no existe, está vacío o no se
/// pudo desproteger (otro usuario de Windows, archivo corrupto).
#[must_use]
pub(crate) fn cargar_protegido_en(directorio: &Path, archivo: &str) -> Option<String> {
    let contenido = fs::read(directorio.join(archivo)).ok()?;
    #[cfg(all(windows, feature = "cifrado-secreto-dispositivo"))]
    if dpapi::es_protegido(&contenido) {
        return dpapi::desproteger(&contenido);
    }
    texto_plano_no_vacio(contenido)
}

fn texto_plano_no_vacio(contenido: Vec<u8>) -> Option<String> {
    let texto = String::from_utf8(contenido).ok()?;
    let texto = texto.trim();
    if texto.is_empty() {
        None
    } else {
        Some(texto.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn directorio_temporal() -> PathBuf {
        std::env::temp_dir().join(format!(
            "control-acceso-dispositivo-nube-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("reloj válido")
                .as_nanos()
        ))
    }

    const ARCHIVO: &str = "prueba.clave";

    #[test]
    fn guarda_y_recupera_el_contenido() {
        let directorio = directorio_temporal();

        guardar_protegido_en(&directorio, ARCHIVO, "contenido-de-prueba").expect("se guarda");

        assert_eq!(
            cargar_protegido_en(&directorio, ARCHIVO).as_deref(),
            Some("contenido-de-prueba")
        );
        assert!(!directorio.join(format!("{ARCHIVO}.tmp")).exists());
        let _ = fs::remove_dir_all(directorio);
    }

    #[test]
    fn ausente_es_none_no_error() {
        assert_eq!(cargar_protegido_en(&directorio_temporal(), ARCHIVO), None);
    }

    #[test]
    fn un_archivo_vacio_cuenta_como_ausente() {
        let directorio = directorio_temporal();
        fs::create_dir_all(&directorio).expect("se crea el directorio");
        fs::write(directorio.join(ARCHIVO), " \n").expect("se escribe a mano");

        assert_eq!(cargar_protegido_en(&directorio, ARCHIVO), None);
        let _ = fs::remove_dir_all(directorio);
    }

    #[cfg(all(windows, feature = "cifrado-secreto-dispositivo"))]
    #[test]
    fn con_el_feature_activo_el_archivo_en_disco_no_queda_en_texto_plano() {
        let directorio = directorio_temporal();

        guardar_protegido_en(&directorio, ARCHIVO, "contenido-de-prueba").expect("se guarda");
        let crudo = fs::read(directorio.join(ARCHIVO)).expect("se lee el archivo crudo");

        assert!(dpapi::es_protegido(&crudo));
        assert!(
            !crudo
                .windows(b"contenido-de-prueba".len())
                .any(|v| v == b"contenido-de-prueba")
        );
        let _ = fs::remove_dir_all(directorio);
    }
}
