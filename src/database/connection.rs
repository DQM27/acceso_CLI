use std::{
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
};

use rusqlite::Connection;

use super::backup::TipoRespaldo;
use super::schema::{SCHEMA_VERSION, SchemaError, initialize_database};

pub const DATABASE_PATH_ENV: &str = "CONTROL_ACCESO_DB";
pub const LOCAL_APP_DATA_ENV: &str = "LOCALAPPDATA";

const APP_DATA_DIRECTORY: &str = "ControlAcceso";
const DATABASE_FILE_NAME: &str = "control_acceso.db";

#[derive(Debug)]
pub enum RutaBaseDatosError {
    VariableNoDisponible {
        variable: &'static str,
    },
    RutaNoAbsoluta {
        variable: &'static str,
        ruta: PathBuf,
    },
    CrearDirectorio {
        ruta: PathBuf,
        origen: io::Error,
    },
}

impl std::fmt::Display for RutaBaseDatosError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VariableNoDisponible { variable } => write!(
                formatter,
                "No se encontró la variable {variable} necesaria para ubicar la base de datos"
            ),
            Self::RutaNoAbsoluta { variable, ruta } => write!(
                formatter,
                "La variable {variable} debe contener una ruta absoluta: {}",
                ruta.display()
            ),
            Self::CrearDirectorio { ruta, origen } => write!(
                formatter,
                "No se pudo crear el directorio de datos {}: {origen}",
                ruta.display()
            ),
        }
    }
}

impl std::error::Error for RutaBaseDatosError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CrearDirectorio { origen, .. } => Some(origen),
            Self::VariableNoDisponible { .. } | Self::RutaNoAbsoluta { .. } => None,
        }
    }
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
pub fn open_database(path: impl AsRef<Path>) -> Result<Connection, SchemaError> {
    let path = path.as_ref();
    let connection = Connection::open(path)?;
    respaldar_antes_de_migrar(&connection, path)?;
    initialize_database(&connection)?;
    Ok(connection)
}

/// Si hay una migración de esquema pendiente, crea un respaldo
/// `TipoRespaldo::PreMigracion` antes de que `initialize_database` la
/// aplique — es obligatorio: si el respaldo falla, esta función devuelve
/// error y la migración nunca corre. No hace nada en una base nueva
/// (`version == 0`, nada que proteger) ni en una ya al día (el caso normal
/// en cada arranque).
fn respaldar_antes_de_migrar(connection: &Connection, path: &Path) -> Result<(), SchemaError> {
    let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version == 0 || version >= SCHEMA_VERSION {
        return Ok(());
    }
    let Some(directorio_base) = path.parent() else {
        return Ok(());
    };
    let directorio_respaldos = directorio_base.join("backups");

    super::backup::crear_respaldo(connection, &directorio_respaldos, TipoRespaldo::PreMigracion)
        .map_err(|error| SchemaError::RespaldoPreMigracionFallido(error.to_string()))?;

    // Best-effort: si la limpieza de respaldos viejos falla, no se bloquea el
    // arranque por eso — sólo el respaldo obligatorio en sí es bloqueante.
    let _ = super::backup::aplicar_retencion(
        &directorio_respaldos,
        TipoRespaldo::PreMigracion,
        super::backup::RETENCION_PRE_MIGRACION,
    );
    Ok(())
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
