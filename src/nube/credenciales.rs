//! Almacenamiento local del secreto de este dispositivo (ver
//! `docs/planes-implementados/plan-persistencia-nube.md`).
//!
//! Sin ningún feature de cifrado sigue en texto plano a propósito, mismo
//! criterio que ya rige el resto de la base local. Dos esquemas de cifrado
//! separados, uno por plataforma:
//!
//! - **Escritorio** (feature `cifrado-secreto-dispositivo`, ver
//!   [`guardar_secreto`]/[`guardar_secreto_en`]): protegido con DPAPI de
//!   Windows (`CryptProtectData`/`CryptUnprotectData`, scope del usuario
//!   actual). No deriva de ningún identificador -- DPAPI liga el blob al
//!   material que gestiona Windows para esa cuenta, no a algo que cualquiera
//!   con acceso al código y a la máquina pueda recalcular (a diferencia del
//!   Machine GUID, que es público). Ver `mod dpapi` más abajo.
//! - **Móvil** (feature `cifrado-secreto-dispositivo-portable`, ver
//!   [`guardar_secreto_en_con_identificador`]): Android ya migró a su
//!   propio Keystore (`SecretoDispositivoStore.kt`) para todo secreto
//!   nuevo -- estas funciones sólo quedan para leer un archivo legado de
//!   antes de esa migración, cifrado con una clave derivada de
//!   `Settings.Secure.ANDROID_ID` (Kotlin se lo pasa, Android no tiene
//!   equivalente al registro de Windows para resolverlo del lado Rust).

use std::{
    fs, io,
    path::{Path, PathBuf},
};

const FILE_NAME: &str = "dispositivo-nube.secret";

#[cfg(feature = "cifrado-secreto-dispositivo-portable")]
mod cifrado {
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Key, Nonce};
    use rand_core::{OsRng, RngCore};
    use sha2::{Digest, Sha256};

    /// Antepuesto al contenido cifrado -- distingue un archivo ya cifrado de
    /// uno viejo en texto plano (de antes de este feature, o escrito por un
    /// dispositivo que todavía no lo activa) sin necesitar un
    /// intento-y-error de descifrado para saberlo.
    const MAGIC: &[u8; 4] = b"BAE1";
    const LARGO_NONCE: usize = 12;

    fn clave(identificador: &str) -> Key<Aes256Gcm> {
        let hash = Sha256::digest(identificador.as_bytes());
        Key::<Aes256Gcm>::try_from(hash.as_slice()).expect("Sha256 produce 32 bytes exactos")
    }

    /// `None` sólo si el cifrado en sí falla (no debería, con una clave de
    /// 32 bytes válida) -- quien llama cae de vuelta a guardar en texto
    /// plano antes que perder el secreto.
    pub(super) fn cifrar(secreto: &str, identificador: &str) -> Option<Vec<u8>> {
        let cipher = Aes256Gcm::new(&clave(identificador));
        let mut nonce_bytes = [0u8; LARGO_NONCE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::try_from(nonce_bytes.as_slice())
            .expect("LARGO_NONCE es el tamaño exacto del nonce");
        let cifrado = cipher.encrypt(&nonce, secreto.as_bytes()).ok()?;

        let mut salida = Vec::with_capacity(MAGIC.len() + LARGO_NONCE + cifrado.len());
        salida.extend_from_slice(MAGIC);
        salida.extend_from_slice(&nonce_bytes);
        salida.extend_from_slice(&cifrado);
        Some(salida)
    }

    /// `true` si `contenido` viene de [`cifrar`] -- lo que no matchea (texto
    /// plano legado, o un archivo de otro dispositivo que todavía no activa
    /// este feature) queda para el camino de texto plano de siempre.
    pub(super) fn es_cifrado(contenido: &[u8]) -> bool {
        contenido.starts_with(MAGIC)
    }

    /// `None` si el archivo está corrupto, o (el caso que motiva todo esto)
    /// `contenido` se cifró con un identificador distinto -- la clave
    /// derivada no calza y el `tag` de GCM no valida, sin distinción posible
    /// entre esos dos casos ni falta que hace.
    pub(super) fn descifrar(contenido: &[u8], identificador: &str) -> Option<String> {
        let resto = contenido.strip_prefix(MAGIC)?;
        if resto.len() < LARGO_NONCE {
            return None;
        }
        let (nonce_bytes, cifrado) = resto.split_at(LARGO_NONCE);
        let nonce = Nonce::try_from(nonce_bytes).ok()?;
        let cipher = Aes256Gcm::new(&clave(identificador));
        let plano = cipher.decrypt(&nonce, cifrado).ok()?;
        String::from_utf8(plano).ok()
    }
}

/// Cifrado real del secreto en escritorio: DPAPI de Windows, no una clave
/// derivada de un identificador público. Ver el doc-comment del módulo.
#[cfg(all(windows, feature = "cifrado-secreto-dispositivo"))]
mod dpapi {
    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::Security::Cryptography::{
        CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
    };

    /// Antepuesto al blob protegido -- distingue esto de un archivo en texto
    /// plano legado o del esquema portable (`BAE1`) sin necesitar
    /// intento-y-error.
    const MAGIC: &[u8; 4] = b"DPA1";

    pub(super) fn es_protegido(contenido: &[u8]) -> bool {
        contenido.starts_with(MAGIC)
    }

    /// `None` sólo si `CryptProtectData` en sí falla -- quien llama cae a
    /// texto plano antes que perder el secreto.
    pub(super) fn proteger(secreto: &str) -> Option<Vec<u8>> {
        let blob = proteger_bytes(secreto.as_bytes())?;
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

/// Resuelve `%LOCALAPPDATA%\ControlAcceso`. `None` si la variable de
/// entorno no está definida o no es una ruta absoluta — mismo criterio de
/// `PreferencesStore::load_default`.
fn directorio_default() -> Option<PathBuf> {
    let root = std::env::var_os(crate::database::connection::LOCAL_APP_DATA_ENV)?;
    let root = PathBuf::from(root);
    if !root.is_absolute() {
        return None;
    }
    Some(root.join("ControlAcceso"))
}

/// Machine GUID de Windows -- ya NO se usa para derivar ninguna clave de
/// cifrado (ver el doc-comment del módulo, DPAPI no lo necesita). Queda
/// exclusivamente para la metadata forense de la activación inicial --
/// `desktop-tauri` la usa ahí, ver `comandos::nube::configurar_dispositivo_inicial`.
/// `None` si no se pudo leer (registro inaccesible, permisos).
#[cfg(feature = "cifrado-secreto-dispositivo")]
pub fn identificador_de_esta_maquina() -> Option<String> {
    machine_uid::get().ok()
}

/// Sólo válido en escritorio, donde `%LOCALAPPDATA%` existe. En Android no
/// hay esa variable de entorno — el lado móvil usa
/// [`guardar_secreto_en_con_identificador`]/[`cargar_secreto_en_con_identificador`]
/// con el directorio que ya le pasa Kotlin para abrir la base `SQLite`
/// (mismo criterio, misma carpeta) y con `Settings.Secure.ANDROID_ID` como
/// identificador.
pub fn guardar_secreto(secreto: &str) -> io::Result<()> {
    let directorio = directorio_default()
        .ok_or_else(|| io::Error::other("no se pudo resolver %LOCALAPPDATA%"))?;
    guardar_secreto_en(&directorio, secreto)
}

/// Ver [`guardar_secreto`] sobre por qué esta versión es sólo de escritorio.
#[must_use]
pub fn cargar_secreto() -> Option<String> {
    cargar_secreto_en(&directorio_default()?)
}

/// Guarda el secreto en `<directorio>/dispositivo-nube.secret`, protegido
/// con DPAPI (feature `cifrado-secreto-dispositivo`, sólo escritorio -- ver
/// [`guardar_secreto_en_con_identificador`] para el equivalente móvil).
pub fn guardar_secreto_en(directorio: &Path, secreto: &str) -> io::Result<()> {
    fs::create_dir_all(directorio)?;
    let secreto = secreto.trim();
    #[cfg(all(windows, feature = "cifrado-secreto-dispositivo"))]
    if let Some(protegido) = dpapi::proteger(secreto) {
        return fs::write(directorio.join(FILE_NAME), protegido);
    }
    fs::write(directorio.join(FILE_NAME), secreto)
}

/// Ver [`guardar_secreto_en`]. Lee tanto un archivo protegido con DPAPI por
/// esta misma cuenta de Windows como uno en texto plano legado.
#[must_use]
pub fn cargar_secreto_en(directorio: &Path) -> Option<String> {
    let contenido = fs::read(directorio.join(FILE_NAME)).ok()?;
    #[cfg(all(windows, feature = "cifrado-secreto-dispositivo"))]
    if dpapi::es_protegido(&contenido) {
        return dpapi::desproteger(&contenido);
    }
    secreto_de_texto_plano(contenido)
}

/// Igual que [`guardar_secreto_en`], pero con un identificador de
/// dispositivo explícito en vez de resolverlo internamente -- lo que usa el
/// lado móvil, donde ese identificador (`Settings.Secure.ANDROID_ID`) sólo
/// Kotlin puede leerlo. Sin el feature `cifrado-secreto-dispositivo-portable`
/// activo, `identificador` se ignora y el comportamiento es texto plano de
/// siempre -- misma lógica que ya tenía [`guardar_secreto_en`], para que
/// quien llama (`application::nube`) no tenga que conocer el feature.
pub fn guardar_secreto_en_con_identificador(
    directorio: &Path,
    secreto: &str,
    #[cfg_attr(
        not(feature = "cifrado-secreto-dispositivo-portable"),
        allow(unused_variables)
    )]
    identificador: &str,
) -> io::Result<()> {
    fs::create_dir_all(directorio)?;
    let secreto = secreto.trim();
    #[cfg(feature = "cifrado-secreto-dispositivo-portable")]
    if let Some(cifrado) = cifrado::cifrar(secreto, identificador) {
        return fs::write(directorio.join(FILE_NAME), cifrado);
    }
    fs::write(directorio.join(FILE_NAME), secreto)
}

/// Ver [`guardar_secreto_en_con_identificador`]. Lee tanto un archivo
/// cifrado con este mismo identificador como uno en texto plano legado.
#[must_use]
pub fn cargar_secreto_en_con_identificador(
    directorio: &Path,
    #[cfg_attr(
        not(feature = "cifrado-secreto-dispositivo-portable"),
        allow(unused_variables)
    )]
    identificador: &str,
) -> Option<String> {
    let contenido = fs::read(directorio.join(FILE_NAME)).ok()?;
    #[cfg(feature = "cifrado-secreto-dispositivo-portable")]
    if cifrado::es_cifrado(&contenido) {
        return cifrado::descifrar(&contenido, identificador);
    }
    secreto_de_texto_plano(contenido)
}

/// Borra `<directorio>/dispositivo-nube.secret` si existe -- para cuando ya
/// se migró su contenido a otro almacén (ver Android Keystore en
/// `SecretoDispositivoStore.kt`) y no tiene sentido dejar la copia vieja
/// (en texto plano en móvil, ver el doc-comment del módulo) huérfana en
/// disco. `Ok(())` si el archivo ya no existía -- borrar algo que no está
/// no es un error para quien llama.
pub fn borrar_secreto_en(directorio: &Path) -> io::Result<()> {
    match fs::remove_file(directorio.join(FILE_NAME)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn secreto_de_texto_plano(contenido: Vec<u8>) -> Option<String> {
    let secreto = String::from_utf8(contenido).ok()?;
    let secreto = secreto.trim();
    if secreto.is_empty() {
        None
    } else {
        Some(secreto.to_owned())
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

    #[test]
    fn guarda_y_recupera_el_secreto() {
        let directorio = directorio_temporal();

        guardar_secreto_en(&directorio, "s3cr3t0-de-prueba").expect("se guarda");
        let recuperado = cargar_secreto_en(&directorio);

        assert_eq!(recuperado.as_deref(), Some("s3cr3t0-de-prueba"));
        let _ = fs::remove_dir_all(directorio);
    }

    #[test]
    fn recorta_espacios_y_saltos_de_linea_al_guardar() {
        let directorio = directorio_temporal();

        guardar_secreto_en(&directorio, "  s3cr3t0  \n").expect("se guarda");
        let recuperado = cargar_secreto_en(&directorio);

        assert_eq!(recuperado.as_deref(), Some("s3cr3t0"));
        let _ = fs::remove_dir_all(directorio);
    }

    #[test]
    fn ausente_es_none_no_error() {
        let directorio = directorio_temporal();

        assert_eq!(cargar_secreto_en(&directorio), None);
    }

    #[test]
    fn un_archivo_en_texto_plano_legado_se_sigue_leyendo() {
        // Simula un secreto guardado por una versión anterior a este
        // feature (o por un dispositivo que todavía no lo activa) -- no
        // debe quedar huérfano cuando esta misma carpeta la abre una
        // versión con cifrado activo.
        let directorio = directorio_temporal();
        fs::create_dir_all(&directorio).expect("se crea el directorio");
        fs::write(directorio.join(FILE_NAME), "secreto-de-antes\n").expect("se escribe a mano");

        assert_eq!(
            cargar_secreto_en(&directorio).as_deref(),
            Some("secreto-de-antes")
        );
        let _ = fs::remove_dir_all(directorio);
    }

    #[cfg(all(windows, feature = "cifrado-secreto-dispositivo"))]
    #[test]
    fn con_el_feature_activo_el_archivo_en_disco_no_queda_en_texto_plano() {
        let directorio = directorio_temporal();

        guardar_secreto_en(&directorio, "s3cr3t0-de-prueba").expect("se guarda");
        let crudo = fs::read(directorio.join(FILE_NAME)).expect("se lee el archivo crudo");

        assert!(dpapi::es_protegido(&crudo));
        assert!(
            !crudo
                .windows(b"s3cr3t0-de-prueba".len())
                .any(|v| v == b"s3cr3t0-de-prueba")
        );
        let _ = fs::remove_dir_all(directorio);
    }

    #[cfg(feature = "cifrado-secreto-dispositivo-portable")]
    #[test]
    fn guarda_y_recupera_con_un_identificador_explicito() {
        let directorio = directorio_temporal();

        guardar_secreto_en_con_identificador(&directorio, "s3cr3t0-movil", "android-id-de-prueba")
            .expect("se guarda");
        let recuperado = cargar_secreto_en_con_identificador(&directorio, "android-id-de-prueba");

        assert_eq!(recuperado.as_deref(), Some("s3cr3t0-movil"));
        let _ = fs::remove_dir_all(directorio);
    }

    #[cfg(feature = "cifrado-secreto-dispositivo-portable")]
    #[test]
    fn un_identificador_distinto_no_descifra() {
        // El caso real que motiva todo esto: el archivo se copió (backup,
        // clonado, robo) a un dispositivo con otro ANDROID_ID/Machine GUID.
        let directorio = directorio_temporal();

        guardar_secreto_en_con_identificador(&directorio, "s3cr3t0-movil", "id-original")
            .expect("se guarda");
        let recuperado = cargar_secreto_en_con_identificador(&directorio, "id-de-otro-dispositivo");

        assert_eq!(recuperado, None);
        let _ = fs::remove_dir_all(directorio);
    }
}
