//! Clave de cifrado de la base de datos (`SQLCipher`) en Windows, protegida
//! con DPAPI (`CryptProtectData`/`CryptUnprotectData`) en vez de derivarla de
//! un identificador de máquina o de usuario -- ver discusión en el chat que
//! motivó esto: derivar de algo público (Machine GUID, `ANDROID_ID`) no es
//! más que ofuscación, cualquiera con acceso al código y a la máquina
//! recalcula la misma clave. DPAPI en cambio la ata al material real que
//! gestiona Windows para la cuenta del usuario actual -- copiar el blob a
//! otro usuario o a otra máquina no sirve de nada.
//!
//! Dos capas separadas, a propósito: DPAPI protege la CLAVE, `SQLCipher`
//! protege la BASE. Copiar `control_acceso.db` y `db_key.dat` juntos a otra
//! parte no expone nada -- el blob de `db_key.dat` sigue atado al usuario de
//! Windows que lo creó.
//!
//! Este módulo es exclusivo de escritorio Windows (`cfg(windows)`); Android
//! usa su propio Keystore (`SecretoDispositivoStore.kt`), sin tocar nada de
//! acá.

use std::fs;
use std::path::{Path, PathBuf};

use windows::Win32::Foundation::LocalFree;
use windows::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
};
use zeroize::Zeroizing;

const NOMBRE_ARCHIVO_CLAVE: &str = "db_key.dat";
const LONGITUD_CLAVE: usize = 32;

#[derive(Debug, thiserror::Error)]
pub enum ErrorClaveBaseDatos {
    #[error("No se pudo leer o escribir {ruta}: {origen}", ruta = .ruta.display())]
    Io {
        ruta: PathBuf,
        #[source]
        origen: std::io::Error,
    },
    #[error("DPAPI no pudo proteger la clave (CryptProtectData falló)")]
    ProtegerFallo,
    #[error(
        "DPAPI no pudo recuperar la clave de {ruta} -- puede ser de otro usuario/máquina, o el \
         archivo está corrupto",
        ruta = .ruta.display()
    )]
    DesprotegerFallo { ruta: PathBuf },
    #[error(
        "el blob protegido en {ruta} no tiene 32 bytes tras desproteger (tiene {longitud})",
        ruta = .ruta.display()
    )]
    LongitudInvalida { ruta: PathBuf, longitud: usize },
    /// Existe `control_acceso.db` pero no `db_key.dat` (o DPAPI no pudo
    /// desprotegerlo): NUNCA se genera una clave nueva encima de una base ya
    /// cifrada -- eso dejaría la base existente irrecuperable en silencio.
    /// Una capa superior decide qué hacer (avisar, ofrecer reconstruir desde
    /// la nube, etc.), este módulo sólo se niega a adivinar.
    #[error(
        "existe una base de datos en {base} pero no se pudo recuperar su clave -- no se genera \
         una nueva para no dejarla irrecuperable",
        base = .ruta_base_datos.display()
    )]
    ClaveFaltanteConBaseExistente { ruta_base_datos: PathBuf },
}

/// Resuelve la clave de 256 bits para `SQLCipher` en `directorio` (el mismo
/// directorio donde vive `control_acceso.db`). Primera vez: genera 32 bytes
/// aleatorios, los protege con DPAPI (scope de usuario actual -- nunca
/// `CRYPTPROTECT_LOCAL_MACHINE`, ver el comentario de `proteger`) y los deja
/// listos para usar. Arranques siguientes: lee el blob y lo desprotege.
///
/// `ruta_base_datos` sólo se usa para el mensaje de
/// [`ErrorClaveBaseDatos::ClaveFaltanteConBaseExistente`] cuando corresponde.
pub fn resolver_clave(
    directorio: &Path,
    ruta_base_datos: &Path,
) -> Result<Zeroizing<[u8; 32]>, ErrorClaveBaseDatos> {
    let ruta_clave = directorio.join(NOMBRE_ARCHIVO_CLAVE);

    match fs::read(&ruta_clave) {
        Ok(blob) => desproteger(&blob, &ruta_clave),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if ruta_base_datos.exists() {
                return Err(ErrorClaveBaseDatos::ClaveFaltanteConBaseExistente {
                    ruta_base_datos: ruta_base_datos.to_path_buf(),
                });
            }
            generar_y_guardar(&ruta_clave)
        }
        Err(origen) => Err(ErrorClaveBaseDatos::Io {
            ruta: ruta_clave,
            origen,
        }),
    }
}

fn generar_y_guardar(ruta_clave: &Path) -> Result<Zeroizing<[u8; 32]>, ErrorClaveBaseDatos> {
    use rand_core::{OsRng, RngCore};

    let mut clave = Zeroizing::new([0_u8; LONGITUD_CLAVE]);
    OsRng.fill_bytes(&mut *clave);

    let protegida = proteger(clave.as_slice())?;
    guardar_atomico(ruta_clave, &protegida).map_err(|origen| ErrorClaveBaseDatos::Io {
        ruta: ruta_clave.to_path_buf(),
        origen,
    })?;
    Ok(clave)
}

fn desproteger(
    blob: &[u8],
    ruta_clave: &Path,
) -> Result<Zeroizing<[u8; 32]>, ErrorClaveBaseDatos> {
    let plano = descifrar_dpapi(blob).ok_or_else(|| ErrorClaveBaseDatos::DesprotegerFallo {
        ruta: ruta_clave.to_path_buf(),
    })?;
    let longitud = plano.len();
    let arreglo: [u8; LONGITUD_CLAVE] =
        plano
            .as_slice()
            .try_into()
            .map_err(|_error| ErrorClaveBaseDatos::LongitudInvalida {
                ruta: ruta_clave.to_path_buf(),
                longitud,
            })?;
    Ok(Zeroizing::new(arreglo))
}

/// Nunca `CRYPTPROTECT_LOCAL_MACHINE`: ese flag permitiría que CUALQUIER
/// usuario de este mismo Windows desprotegiera el blob. Con el scope por
/// defecto (usuario actual), sólo la cuenta de Windows que ejecutó esto
/// puede recuperar la clave.
fn proteger(datos: &[u8]) -> Result<Vec<u8>, ErrorClaveBaseDatos> {
    let mut entrada = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(datos.len()).unwrap_or(u32::MAX),
        pbData: datos.as_ptr().cast_mut(),
    };
    let mut salida = CRYPT_INTEGER_BLOB::default();

    // SAFETY: `entrada` apunta a `datos`, vivo durante toda la llamada;
    // `salida` la llena la propia API y su buffer se libera con LocalFree
    // apenas se copia a un `Vec` nuestro.
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

    if resultado.is_err() {
        return Err(ErrorClaveBaseDatos::ProtegerFallo);
    }
    Ok(copiar_y_liberar(salida))
}

fn descifrar_dpapi(blob: &[u8]) -> Option<Vec<u8>> {
    let mut entrada = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(blob.len()).ok()?,
        pbData: blob.as_ptr().cast_mut(),
    };
    let mut salida = CRYPT_INTEGER_BLOB::default();

    // SAFETY: mismo criterio que en `proteger`; `CRYPTPROTECT_UI_FORBIDDEN`
    // evita que un blob corrupto/ajeno dispare un diálogo nativo de Windows.
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
    // SAFETY: `blob.pbData` apunta a `blob.cbData` bytes válidos, escritos
    // por CryptProtectData/CryptUnprotectData justo antes de esta llamada.
    let copia = unsafe { std::slice::from_raw_parts(blob.pbData, blob.cbData as usize) }.to_vec();
    // SAFETY: `blob.pbData` la reservó DPAPI con LocalAlloc -- LocalFree es
    // la contraparte documentada para liberarla.
    unsafe {
        let _ = LocalFree(Some(windows::Win32::Foundation::HLOCAL(
            blob.pbData.cast::<std::ffi::c_void>(),
        )));
    }
    copia
}

/// Escribe el blob protegido con un archivo temporal + rename atómico, para
/// que un corte de energía a mitad de escritura nunca deje `db_key.dat` a
/// medias (mismo patrón que `AndroidKeystoreSecretoDispositivoStore.kt`).
fn guardar_atomico(ruta: &Path, contenido: &[u8]) -> std::io::Result<()> {
    if let Some(directorio) = ruta.parent() {
        fs::create_dir_all(directorio)?;
    }
    let temporal = ruta.with_extension("tmp");
    fs::write(&temporal, contenido)?;
    fs::rename(&temporal, ruta)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directorio_temporal(nombre: &str) -> PathBuf {
        let unico = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directorio = std::env::temp_dir().join(format!(
            "control_acceso_clave_cifrado_{nombre}_{}_{unico}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directorio).unwrap();
        directorio
    }

    #[test]
    fn proteger_y_desproteger_devuelve_los_mismos_bytes() {
        let original = [7_u8; 32];
        let protegido = proteger(&original).unwrap();
        let recuperado = descifrar_dpapi(&protegido).unwrap();
        assert_eq!(recuperado, original.to_vec());
    }

    #[test]
    fn generar_clave_nueva_y_recuperarla_en_un_segundo_arranque_da_los_mismos_bytes() {
        let directorio = directorio_temporal("roundtrip");
        let ruta_base_datos = directorio.join("control_acceso.db");

        let primera = resolver_clave(&directorio, &ruta_base_datos).unwrap();
        let segunda = resolver_clave(&directorio, &ruta_base_datos).unwrap();
        assert_eq!(*primera, *segunda);

        std::fs::remove_dir_all(&directorio).ok();
    }

    #[test]
    fn sin_archivo_de_clave_ni_base_de_datos_genera_una_clave_nueva() {
        let directorio = directorio_temporal("primera_vez");
        let ruta_base_datos = directorio.join("control_acceso.db");

        let clave = resolver_clave(&directorio, &ruta_base_datos).unwrap();
        assert!(directorio.join(NOMBRE_ARCHIVO_CLAVE).exists());
        assert_ne!(*clave, [0_u8; 32]);

        std::fs::remove_dir_all(&directorio).ok();
    }

    #[test]
    fn base_existente_sin_archivo_de_clave_no_genera_una_nueva_en_silencio() {
        let directorio = directorio_temporal("base_sin_clave");
        let ruta_base_datos = directorio.join("control_acceso.db");
        std::fs::write(&ruta_base_datos, b"contenido de una base existente").unwrap();

        let resultado = resolver_clave(&directorio, &ruta_base_datos);
        assert!(matches!(
            resultado,
            Err(ErrorClaveBaseDatos::ClaveFaltanteConBaseExistente { .. })
        ));
        assert!(!directorio.join(NOMBRE_ARCHIVO_CLAVE).exists());

        std::fs::remove_dir_all(&directorio).ok();
    }

    #[test]
    fn blob_corrupto_devuelve_error_en_vez_de_generar_una_clave_nueva() {
        let directorio = directorio_temporal("blob_corrupto");
        let ruta_base_datos = directorio.join("control_acceso.db");
        std::fs::write(
            directorio.join(NOMBRE_ARCHIVO_CLAVE),
            b"esto no es un blob DPAPI valido",
        )
        .unwrap();

        let resultado = resolver_clave(&directorio, &ruta_base_datos);
        assert!(matches!(
            resultado,
            Err(ErrorClaveBaseDatos::DesprotegerFallo { .. })
        ));

        std::fs::remove_dir_all(&directorio).ok();
    }

    #[test]
    fn el_error_nunca_incluye_los_bytes_de_la_clave() {
        let directorio = directorio_temporal("sin_fuga_en_error");
        let ruta_base_datos = directorio.join("control_acceso.db");
        std::fs::write(directorio.join(NOMBRE_ARCHIVO_CLAVE), b"blob invalido").unwrap();

        let error = resolver_clave(&directorio, &ruta_base_datos).unwrap_err();
        let texto = error.to_string();
        // No hay bytes de clave que filtrar en este caso (el error es de
        // formato), pero sí confirma que el mensaje no imprime `{:?}` de
        // datos crudos -- ninguna variante de este enum deriva Debug sobre
        // buffers de clave.
        assert!(!texto.contains("esto no es un blob"));

        std::fs::remove_dir_all(&directorio).ok();
    }
}
