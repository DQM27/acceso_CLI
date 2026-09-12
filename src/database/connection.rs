use std::{
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
};

use rusqlite::Connection;

use super::schema::{SchemaError, initialize_database, verificar_archivo_propio};

pub const DATABASE_PATH_ENV: &str = "CONTROL_ACCESO_DB";
pub const LOCAL_APP_DATA_ENV: &str = "LOCALAPPDATA";

const APP_DATA_DIRECTORY: &str = "ControlAcceso";
const DATABASE_FILE_NAME: &str = "control_acceso.db";

#[derive(Debug, thiserror::Error)]
pub enum RutaBaseDatosError {
    #[error("No se encontró la variable {variable} necesaria para ubicar la base de datos")]
    VariableNoDisponible { variable: &'static str },
    #[error(
        "La variable {variable} debe contener una ruta absoluta: {}",
        .ruta.display()
    )]
    RutaNoAbsoluta {
        variable: &'static str,
        ruta: PathBuf,
    },
    #[error("No se pudo crear el directorio de datos {}: {origen}", .ruta.display())]
    CrearDirectorio {
        ruta: PathBuf,
        #[source]
        origen: io::Error,
    },
}

pub fn ruta_base_datos() -> Result<PathBuf, RutaBaseDatosError> {
    let ruta = resolver_ruta_base_datos(
        std::env::var_os(DATABASE_PATH_ENV),
        std::env::var_os(LOCAL_APP_DATA_ENV),
    )?;
    preparar_directorio(&ruta)?;
    Ok(ruta)
}

fn resolver_ruta_base_datos(
    ruta_configurada: Option<OsString>,
    directorio_local: Option<OsString>,
) -> Result<PathBuf, RutaBaseDatosError> {
    if let Some(ruta_configurada) = ruta_configurada {
        return exigir_ruta_absoluta(DATABASE_PATH_ENV, PathBuf::from(ruta_configurada));
    }

    let directorio_local =
        directorio_local
            .map(PathBuf::from)
            .ok_or(RutaBaseDatosError::VariableNoDisponible {
                variable: LOCAL_APP_DATA_ENV,
            })?;
    let directorio_local = exigir_ruta_absoluta(LOCAL_APP_DATA_ENV, directorio_local)?;

    Ok(directorio_local
        .join(APP_DATA_DIRECTORY)
        .join(DATABASE_FILE_NAME))
}

fn exigir_ruta_absoluta(
    variable: &'static str,
    ruta: PathBuf,
) -> Result<PathBuf, RutaBaseDatosError> {
    if ruta.is_absolute() {
        Ok(ruta)
    } else {
        Err(RutaBaseDatosError::RutaNoAbsoluta { variable, ruta })
    }
}

fn preparar_directorio(ruta_base_datos: &Path) -> Result<(), RutaBaseDatosError> {
    let Some(directorio) = ruta_base_datos.parent() else {
        return Ok(());
    };

    fs::create_dir_all(directorio).map_err(|origen| RutaBaseDatosError::CrearDirectorio {
        ruta: directorio.to_path_buf(),
        origen,
    })
}

/// Abre la base productiva y aplica toda su inicialización en una única ruta.
///
/// Rama experimental de cifrado: los respaldos locales previos a migración se
/// eliminaron deliberadamente. La recuperación se delega a la nube y evitamos
/// crear copias `SQLite` en claro alrededor de una base cifrada.
pub fn open_database(path: impl AsRef<Path>) -> Result<Connection, SchemaError> {
    abrir_conexion(path, None)
}

/// Igual que [`open_database`], pero cifrada con `SQLCipher` usando `clave`
/// como clave binaria cruda (no una passphrase -- `SQLCipher` se salta la
/// derivación PBKDF2 de una vez, ver `PRAGMA key = "x'...'"` en su
/// documentación). Quien llama resuelve y protege esa clave (ver
/// `desktop/src-tauri/src/clave_cifrado.rs` para el esquema con DPAPI en
/// escritorio) -- este módulo sólo la aplica.
pub fn open_database_cifrada(
    path: impl AsRef<Path>,
    clave: &[u8; 32],
) -> Result<Connection, SchemaError> {
    abrir_conexion(path, Some(clave))
}

/// Aplica la clave de `SQLCipher` a una conexión recién abierta con
/// `Connection::open` -- para conexiones secundarias al mismo archivo que no
/// deben repetir `initialize_database` (ya migrada por la conexión
/// principal). Debe llamarse antes de cualquier otra operación sobre la
/// conexión: sin la clave, `SQLCipher` ni siquiera puede leer el schema.
pub fn aplicar_clave(connection: &Connection, clave: &[u8; 32]) -> rusqlite::Result<()> {
    connection.pragma_update(None, "key", format!("x'{}'", clave_a_hex(clave)))
}

fn abrir_conexion(
    path: impl AsRef<Path>,
    clave: Option<&[u8; 32]>,
) -> Result<Connection, SchemaError> {
    let connection = Connection::open(path)?;
    if let Some(clave) = clave {
        aplicar_clave(&connection, clave)?;
    }
    verificar_archivo_propio(&connection)?;
    initialize_database(&connection)?;
    Ok(connection)
}

/// Pragmas por-conexión que `initialize_database` ya fija en la conexión
/// principal (`fijar_pragmas_iniciales`, ver `schema.rs`) -- salvo
/// `journal_mode`, que es una propiedad del archivo (no de la conexión) y
/// ya queda heredada por cualquier conexión nueva una vez que la principal
/// la activó una sola vez. `foreign_keys`, `synchronous`, `trusted_schema`
/// y `secure_delete` sí son por-conexión: una conexión que sólo hace
/// `Connection::open()` sin repetir esto arranca con los valores por
/// defecto de `SQLite` (`foreign_keys` OFF, entre otros), no con la
/// configuración real de la app. Común a las dos variantes de abajo --
/// `query_only` NO va acá porque sólo aplica a la de sólo lectura.
const PRAGMAS_CONEXION_SECUNDARIA_BASE: &str = "
    PRAGMA foreign_keys = ON;
    PRAGMA busy_timeout = 5000;
    PRAGMA synchronous = EXTRA;
    PRAGMA trusted_schema = OFF;
    PRAGMA secure_delete = FAST;
    ";

/// Deja la conexión lista (clave si corresponde + pragmas base) sin decidir
/// `query_only` -- lo hacen las dos funciones públicas de abajo, cada una
/// agregando (o no) esa única línea después de esta base común.
fn abrir_conexion_secundaria_base(
    path: impl AsRef<Path>,
    clave: Option<&[u8; 32]>,
) -> Result<Connection, SchemaError> {
    let connection = Connection::open(path)?;
    if let Some(clave) = clave {
        aplicar_clave(&connection, clave)?;
    }
    connection.execute_batch(PRAGMAS_CONEXION_SECUNDARIA_BASE)?;
    Ok(connection)
}

/// Abre una conexión adicional de **sólo lectura** al mismo archivo que ya
/// inicializó [`open_database`]/[`open_database_cifrada`] -- pensada para
/// trabajo en un hilo aparte (exportar) que no debe competir por el
/// candado de escritura de la conexión principal
/// (`docs/pendientes.md` sobre el hilo de exportación en TUI/CLI). Agrega
/// `PRAGMA query_only = ON` sobre la base común: `SQLite` rechaza cualquier
/// escritura en esta conexión, incluso dentro de una transacción -- si
/// algún llamador necesita escribir (ej. un hilo de sincronización), debe
/// usar [`abrir_conexion_secundaria_escritura`] en su lugar. A diferencia
/// de `open_database`/`open_database_cifrada`, NO corre
/// `initialize_database` (la conexión principal ya migró el archivo) ni
/// vuelve a validarlo. `clave` en `None` para una base sin cifrar -- hoy el
/// único caso real de TUI/CLI, que nunca abren una base cifrada; se deja el
/// parámetro para no tener que tocar a los llamadores otra vez si eso
/// cambia.
pub fn abrir_conexion_secundaria(
    path: impl AsRef<Path>,
    clave: Option<&[u8; 32]>,
) -> Result<Connection, SchemaError> {
    let connection = abrir_conexion_secundaria_base(path, clave)?;
    connection.execute_batch("PRAGMA query_only = ON;")?;
    Ok(connection)
}

/// Igual que [`abrir_conexion_secundaria`] (misma base de pragmas, sin
/// `initialize_database`), pero sin `query_only`: para un hilo aparte que sí
/// necesita escribir -- hoy sólo `GuiState::conexion_secundaria` en
/// `desktop/src-tauri/src/estado.rs`, que sincroniza con la nube
/// (`drenar_cola`, `cerrar_ingreso_remoto`) desde una conexión propia para no
/// competir por el candado de la principal. Unifica lo que antes era una
/// implementación separada en escritorio con pragmas incompletos (le
/// faltaban `foreign_keys`/`synchronous`/`trusted_schema`/`secure_delete`,
/// ver `docs/auditorias/auditoria-rendimiento-core-rust-2026-09-10.md`,
/// hallazgo R-07).
pub fn abrir_conexion_secundaria_escritura(
    path: impl AsRef<Path>,
    clave: Option<&[u8; 32]>,
) -> Result<Connection, SchemaError> {
    abrir_conexion_secundaria_base(path, clave)
}

fn clave_a_hex(clave: &[u8; 32]) -> String {
    use std::fmt::Write;
    clave
        .iter()
        .fold(String::with_capacity(64), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static SECUENCIA: AtomicU64 = AtomicU64::new(0);

    fn directorio_temporal(nombre: &str) -> PathBuf {
        let numero = SECUENCIA.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "control_acceso_ruta_{nombre}_{}_{numero}",
            std::process::id()
        ))
    }

    #[test]
    fn usa_local_app_data_por_defecto() {
        let local = directorio_temporal("local");
        let ruta = resolver_ruta_base_datos(None, Some(local.clone().into_os_string())).unwrap();
        assert_eq!(
            ruta,
            local.join(APP_DATA_DIRECTORY).join(DATABASE_FILE_NAME)
        );
    }

    #[test]
    fn ruta_configurada_absoluta_tiene_prioridad() {
        let configurada = directorio_temporal("configurada").join("personalizada.db");
        let local = directorio_temporal("ignorada");
        let ruta = resolver_ruta_base_datos(
            Some(configurada.clone().into_os_string()),
            Some(local.into_os_string()),
        )
        .unwrap();
        assert_eq!(ruta, configurada);
    }

    #[test]
    fn rechaza_ruta_configurada_relativa() {
        let resultado = resolver_ruta_base_datos(
            Some(OsString::from("otra.db")),
            Some(directorio_temporal("local").into_os_string()),
        );
        assert!(matches!(
            resultado,
            Err(RutaBaseDatosError::RutaNoAbsoluta {
                variable: DATABASE_PATH_ENV,
                ..
            })
        ));
    }

    #[test]
    fn falla_si_local_app_data_no_esta_disponible() {
        let resultado = resolver_ruta_base_datos(None, None);
        assert!(matches!(
            resultado,
            Err(RutaBaseDatosError::VariableNoDisponible {
                variable: LOCAL_APP_DATA_ENV
            })
        ));
    }

    #[test]
    fn crea_el_directorio_padre_antes_de_abrir_sqlite() {
        let raiz = directorio_temporal("crear");
        let ruta = raiz.join("datos").join(DATABASE_FILE_NAME);
        preparar_directorio(&ruta).unwrap();
        assert!(ruta.parent().unwrap().is_dir());
        fs::remove_dir_all(&raiz).unwrap();
    }
}
