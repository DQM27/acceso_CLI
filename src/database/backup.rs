//! Motor de creación y validación de respaldos (Fase 1 de `docs/plan-respaldos.md`).
//!
//! Usa la [Online Backup API de SQLite](https://www.sqlite.org/backup.html) vía
//! `rusqlite::backup` — nunca una copia directa del archivo mientras SQLite está
//! abierto. Este módulo no sabe nada de la TUI ni de `AppCore`: recibe una
//! `&Connection` ya abierta y un directorio destino, y devuelve tipos neutrales.
//!
//! El archivo se escribe primero con extensión `.partial`, se valida por
//! separado (`integrity_check` + `foreign_key_check`, ninguno de los dos cubre
//! lo que cubre el otro) y sólo si pasa ambas verificaciones se renombra al
//! nombre definitivo. Un respaldo inválido nunca queda publicado: el
//! `.partial` se borra y `crear_respaldo` devuelve error.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, NaiveDateTime, Utc};
use rusqlite::{Connection, OpenFlags};

use super::schema::SCHEMA_VERSION;

const PREFIJO_ARCHIVO: &str = "control_acceso";
const FORMATO_FECHA: &str = "%Y-%m-%d_%H%M%S";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoRespaldo {
    Manual,
    Automatico,
    PreMigracion,
    PreRestauracion,
}

impl TipoRespaldo {
    fn sufijo(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Automatico => "automatico",
            Self::PreMigracion => "pre_migracion",
            Self::PreRestauracion => "pre_restauracion",
        }
    }

}

/// Metadatos de un respaldo ya existente, obtenidos del sistema de archivos
/// y del nombre del archivo — no requiere abrir SQLite. Barato de calcular
/// para listar; la validación real es una operación aparte y más costosa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RespaldoResumen {
    pub ruta: PathBuf,
    pub creado_en: DateTime<Utc>,
    pub tipo: TipoRespaldo,
    pub tamano_bytes: u64,
}

/// Resultado de abrir un respaldo aparte y verificarlo de verdad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultadoValidacion {
    Valido { version_esquema: i64 },
    Invalido(String),
    /// El archivo es una base de Control Acceso válida, pero de una versión
    /// de esquema que esta versión de la aplicación no reconoce.
    EsquemaIncompatible { version_encontrada: i64 },
}

impl ResultadoValidacion {
    pub fn es_valido(&self) -> bool {
        matches!(self, Self::Valido { .. })
    }
}

#[derive(Debug)]
pub enum RespaldoError {
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    /// El respaldo recién creado no pasó su propia validación; el `.partial`
    /// ya fue eliminado antes de devolver este error.
    ValidacionFallida(ResultadoValidacion),
}

impl std::fmt::Display for RespaldoError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlite(error) => write!(formatter, "Error de SQLite: {error}"),
            Self::Io(error) => write!(formatter, "Error de archivo: {error}"),
            Self::ValidacionFallida(ResultadoValidacion::Invalido(detalle)) => {
                write!(formatter, "El respaldo generado no pasó la verificación: {detalle}")
            }
            Self::ValidacionFallida(ResultadoValidacion::EsquemaIncompatible {
                version_encontrada,
            }) => write!(
                formatter,
                "El respaldo generado quedó en una versión de esquema incompatible ({version_encontrada})"
            ),
            Self::ValidacionFallida(ResultadoValidacion::Valido { .. }) => {
                write!(formatter, "Error interno: validación marcada como fallida pero válida")
            }
        }
    }
}

impl std::error::Error for RespaldoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sqlite(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::ValidacionFallida(_) => None,
        }
    }
}

impl From<rusqlite::Error> for RespaldoError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl From<std::io::Error> for RespaldoError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// Crea un respaldo consistente de `origen` dentro de `directorio_respaldos`
/// (se crea si no existe) usando la Online Backup API. Sólo devuelve `Ok`
/// una vez que el archivo ya pasó `integrity_check` + `foreign_key_check` y
/// fue renombrado a su nombre definitivo; si algo falla, no deja ningún
/// archivo `.partial` atrás.
pub fn crear_respaldo(
    origen: &Connection,
    directorio_respaldos: &Path,
    tipo: TipoRespaldo,
) -> Result<RespaldoResumen, RespaldoError> {
    fs::create_dir_all(directorio_respaldos)?;

    let ahora = Utc::now();
    let nombre_base = format!(
        "{PREFIJO_ARCHIVO}_{}_{}",
        ahora.format(FORMATO_FECHA),
        tipo.sufijo()
    );
    let ruta_parcial = ruta_disponible(directorio_respaldos, &nombre_base, "partial");

    {
        let mut destino = Connection::open(&ruta_parcial)?;
        let respaldo = rusqlite::backup::Backup::new(origen, &mut destino)?;
        respaldo.run_to_completion(100, Duration::from_millis(10), None)?;
    } // `destino` se cierra aquí, antes de reabrir el mismo archivo para validarlo.

    let validacion = validar_respaldo(&ruta_parcial)?;
    let ResultadoValidacion::Valido { .. } = &validacion else {
        let _ = fs::remove_file(&ruta_parcial);
        return Err(RespaldoError::ValidacionFallida(validacion));
    };

    let ruta_final = ruta_disponible(directorio_respaldos, &nombre_base, "db");
    fs::rename(&ruta_parcial, &ruta_final)?;

    Ok(RespaldoResumen {
        tamano_bytes: fs::metadata(&ruta_final)?.len(),
        ruta: ruta_final,
        creado_en: ahora,
        tipo,
    })
}

/// Abre `ruta` en modo sólo lectura (nunca crea ni modifica el archivo) y
/// corre las dos verificaciones que recomienda la documentación de SQLite:
/// `integrity_check` (estructura de páginas) y `foreign_key_check`
/// (relaciones entre tablas, que `integrity_check` no cubre).
pub fn validar_respaldo(ruta: &Path) -> Result<ResultadoValidacion, RespaldoError> {
    let connection = Connection::open_with_flags(ruta, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    let integridad: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integridad != "ok" {
        return Ok(ResultadoValidacion::Invalido(integridad));
    }

    let mut violaciones_fk = connection.prepare("PRAGMA foreign_key_check")?;
    let hay_violaciones = violaciones_fk.query([])?.next()?.is_some();
    if hay_violaciones {
        return Ok(ResultadoValidacion::Invalido(
            "hay referencias de clave foránea inválidas".to_owned(),
        ));
    }

    let version_esquema: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version_esquema > SCHEMA_VERSION {
        return Ok(ResultadoValidacion::EsquemaIncompatible {
            version_encontrada: version_esquema,
        });
    }

    Ok(ResultadoValidacion::Valido {
        version_esquema,
    })
}

/// Lista los respaldos ya publicados en `directorio_respaldos` (nunca los
/// `.partial`, que son un estado intermedio, no un respaldo real). No abre
/// SQLite — sólo lee el sistema de archivos y el nombre de cada archivo;
/// para saber si un respaldo sigue siendo válido hay que llamar
/// `validar_respaldo` aparte.
pub fn listar_respaldos(directorio_respaldos: &Path) -> Result<Vec<RespaldoResumen>, RespaldoError> {
    if !directorio_respaldos.exists() {
        return Ok(Vec::new());
    }
    let mut respaldos = Vec::new();
    for entrada in fs::read_dir(directorio_respaldos)? {
        let entrada = entrada?;
        let ruta = entrada.path();
        if ruta.extension().and_then(|e| e.to_str()) != Some("db") {
            continue;
        }
        let Some(resumen) = interpretar_nombre(&ruta) else {
            continue;
        };
        respaldos.push(resumen);
    }
    respaldos.sort_by_key(|r| std::cmp::Reverse(r.creado_en));
    Ok(respaldos)
}

/// Reconstruye fecha/tipo a partir del nombre de archivo que arma
/// `crear_respaldo` (`control_acceso_{fecha}_{hora}_{tipo}.db`, con un
/// posible sufijo numérico `_2`, `_3`... si hubo colisión). El tamaño sale
/// del sistema de archivos, no del nombre.
///
/// `fecha_hora` tiene ancho fijo (`YYYY-MM-DD_HHMMSS`, 17 caracteres), lo
/// que permite cortarlo por posición en vez de por separador — necesario
/// porque dos de los cuatro sufijos de tipo (`pre_migracion`,
/// `pre_restauracion`) ya traen un guion bajo propio y romperían un split
/// ingenuo por `_`.
fn interpretar_nombre(ruta: &Path) -> Option<RespaldoResumen> {
    const ANCHO_FECHA_HORA: usize = 17;
    let nombre = ruta.file_stem()?.to_str()?;
    let resto = nombre.strip_prefix(PREFIJO_ARCHIVO)?.strip_prefix('_')?;
    let fecha_hora = resto.get(..ANCHO_FECHA_HORA)?;
    let resto = resto.get(ANCHO_FECHA_HORA..)?.strip_prefix('_')?;

    let tipo = [
        TipoRespaldo::Manual,
        TipoRespaldo::Automatico,
        TipoRespaldo::PreMigracion,
        TipoRespaldo::PreRestauracion,
    ]
    .into_iter()
    .find(|tipo| resto == tipo.sufijo() || resto.starts_with(&format!("{}_", tipo.sufijo())))?;

    let creado_en = NaiveDateTime::parse_from_str(fecha_hora, FORMATO_FECHA)
        .ok()?
        .and_utc();
    let tamano_bytes = fs::metadata(ruta).ok()?.len();
    Some(RespaldoResumen {
        ruta: ruta.to_path_buf(),
        creado_en,
        tipo,
        tamano_bytes,
    })
}

/// Nunca sobrescribe un archivo existente: si el nombre base ya está
/// ocupado, agrega un sufijo numérico creciente.
fn ruta_disponible(directorio: &Path, nombre_base: &str, extension: &str) -> PathBuf {
    let candidato = directorio.join(format!("{nombre_base}.{extension}"));
    if !candidato.exists() {
        return candidato;
    }
    (2..).find_map(|n| {
        let candidato = directorio.join(format!("{nombre_base}_{n}.{extension}"));
        (!candidato.exists()).then_some(candidato)
    }).expect("el rango de sufijos numéricos es infinito")
}
