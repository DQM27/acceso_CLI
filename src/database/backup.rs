//! Motor de creación y validación de respaldos (ver `docs/pendientes.md`).
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

use super::schema::{SCHEMA_VERSION, SchemaError, initialize_database};

const PREFIJO_ARCHIVO: &str = "control_acceso";
const FORMATO_FECHA: &str = "%Y-%m-%d_%H%M%S";

/// Cuántos respaldos automáticos diarios se conservan antes de que
/// `aplicar_retencion` empiece a borrar los más viejos.
pub const RETENCION_AUTOMATICOS: usize = 7;
/// Cuántos respaldos previos a una migración de esquema se conservan.
pub const RETENCION_PRE_MIGRACION: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum TipoRespaldo {
    Manual,
    Automatico,
    PreMigracion,
    PreRestauracion,
    /// Disparado por un flag de línea de comandos (p. ej. `--reset-root`) en vez de
    /// desde la TUI — se distingue de `Manual` para que quede claro, al mirar la
    /// lista de respaldos, que fue una intervención fuera de la app normal.
    PorFlag,
}

impl TipoRespaldo {
    fn sufijo(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Automatico => "automatico",
            Self::PreMigracion => "pre_migracion",
            Self::PreRestauracion => "pre_restauracion",
            Self::PorFlag => "por_flag",
        }
    }
}

/// Metadatos de un respaldo ya existente, obtenidos del sistema de archivos
/// y del nombre del archivo — no requiere abrir SQLite. Barato de calcular
/// para listar; la validación real es una operación aparte y más costosa.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct RespaldoResumen {
    pub ruta: PathBuf,
    pub creado_en: DateTime<Utc>,
    pub tipo: TipoRespaldo,
    pub tamano_bytes: u64,
}

/// Resultado de abrir un respaldo aparte y verificarlo de verdad.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ResultadoValidacion {
    Valido {
        version_esquema: i64,
    },
    Invalido(String),
    /// El archivo es una base de Control Acceso válida, pero de una versión
    /// de esquema que esta versión de la aplicación no reconoce.
    EsquemaIncompatible {
        version_encontrada: i64,
    },
}

impl ResultadoValidacion {
    pub fn es_valido(&self) -> bool {
        matches!(self, Self::Valido { .. })
    }
}

impl std::fmt::Display for ResultadoValidacion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalido(detalle) => write!(
                formatter,
                "El respaldo generado no pasó la verificación: {detalle}"
            ),
            Self::EsquemaIncompatible { version_encontrada } => write!(
                formatter,
                "El respaldo generado quedó en una versión de esquema incompatible ({version_encontrada})"
            ),
            Self::Valido { .. } => write!(
                formatter,
                "Error interno: validación marcada como fallida pero válida"
            ),
        }
    }
}

fn detalle_ruta_previa(ruta_previa: Option<&Path>) -> String {
    ruta_previa.map_or_else(String::new, |ruta| {
        format!(" (la base anterior puede seguir en {})", ruta.display())
    })
}

#[derive(Debug, thiserror::Error)]
pub enum RespaldoError {
    #[error("Sólo una sesión ROOT activa puede gestionar respaldos")]
    OperacionNoAutorizada,
    #[error("Error de SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Error de archivo: {0}")]
    Io(#[from] std::io::Error),
    /// El respaldo recién creado no pasó su propia validación; el `.partial`
    /// ya fue eliminado antes de devolver este error.
    #[error("{0}")]
    ValidacionFallida(ResultadoValidacion),
    /// `restaurar_respaldo` falló al abrir/verificar la candidata Y el intento
    /// de reinstalar la base anterior también falló — a diferencia del resto
    /// de variantes, el sistema puede haber quedado sin una base activa
    /// consistente en `ruta_activa`. Distinto de un rollback exitoso, que
    /// simplemente reporta `error_original` sin esta variante.
    #[error(
        "La restauración falló ({error_original}) y no se pudo dejar el sistema en un estado consistente. Revise manualmente {}{}",
        .ruta_activa.display(),
        detalle_ruta_previa(.ruta_previa.as_deref())
    )]
    RollbackFallido {
        #[source]
        error_original: Box<RespaldoError>,
        ruta_activa: PathBuf,
        ruta_previa: Option<PathBuf>,
    },
    /// Un archivo temporal (`.partial` inválido) no se pudo borrar después de
    /// un error real — el error original queda adjunto para no perder la
    /// pista, en vez de descartar la falla de limpieza con `let _ =`.
    #[error(
        "{error_original} (además, no se pudo borrar el archivo temporal {})",
        .ruta.display()
    )]
    LimpiezaFallida {
        #[source]
        error_original: Box<RespaldoError>,
        ruta: PathBuf,
    },
}

impl From<SchemaError> for RespaldoError {
    fn from(error: SchemaError) -> Self {
        match error {
            SchemaError::Sqlite(error) => Self::Sqlite(error),
            SchemaError::BaseAjena => Self::ValidacionFallida(ResultadoValidacion::Invalido(
                "el archivo restaurado no es una base de Control Acceso".to_owned(),
            )),
            SchemaError::IntegridadInvalida(detalle) => {
                Self::ValidacionFallida(ResultadoValidacion::Invalido(detalle))
            }
            // No lo produce ningún camino real (initialize_database nunca lo
            // devuelve; sólo open_database lo genera, y restaurar_respaldo no
            // pasa por ahí), pero el match debe seguir siendo exhaustivo.
            SchemaError::RespaldoPreMigracionFallido(detalle) => {
                Self::ValidacionFallida(ResultadoValidacion::Invalido(detalle))
            }
            SchemaError::VersionInesperadaTrasMigrar { encontrada } => {
                Self::ValidacionFallida(ResultadoValidacion::Invalido(format!(
                    "la base quedó en la versión de esquema {encontrada} tras migrar, un estado interno inconsistente"
                )))
            }
            // No lo produce ningún camino real (las 7 tablas se recrean
            // copiando exactamente los mismos datos), pero el match debe
            // seguir siendo exhaustivo — mismo criterio que
            // `RespaldoPreMigracionFallido` arriba.
            SchemaError::MigracionStrictReferenciasInvalidas => {
                Self::ValidacionFallida(ResultadoValidacion::Invalido(
                    "la migración a tablas STRICT dejó filas con una clave foránea inválida"
                        .to_owned(),
                ))
            }
        }
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
        let error_original = RespaldoError::ValidacionFallida(validacion);
        return Err(if fs::remove_file(&ruta_parcial).is_ok() {
            error_original
        } else {
            RespaldoError::LimpiezaFallida {
                error_original: Box::new(error_original),
                ruta: ruta_parcial,
            }
        });
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

/// Reemplaza `ruta_activa` por `ruta_candidata` (ver `docs/pendientes.md`).
///
/// **Debe llamarse con la conexión de `ruta_activa` ya cerrada.** SQLite
/// documenta los riesgos de mover o reemplazar un archivo con una
/// transacción o conexión activa sobre él
/// ([How To Corrupt An SQLite Database File](https://sqlite.org/howtocorrupt.html));
/// en Windows además el propio sistema operativo puede impedir el rename
/// mientras el archivo esté abierto. El llamador es responsable de, en este
/// orden: 1) crear el respaldo `PreRestauracion` de la base activa mientras
/// la conexión sigue abierta (`crear_respaldo` necesita una `&Connection`
/// real), 2) cerrar/descartar esa conexión (`AppCore`), 3) llamar a esta
/// función, 4) si devuelve `Ok`, volver a abrir la base y exigir un login
/// nuevo. `InstanciaGuard` debe seguir vivo durante todo el proceso — esta
/// función no lo toca, es responsabilidad del llamador.
///
/// No destruye la base activa hasta haber copiado la candidata a un
/// temporal en el mismo directorio: si la copia falla, la base activa ni se
/// entera. Si algo falla después del intercambio (migración incompatible,
/// archivo corrupto pese a la validación previa), reinstala automáticamente
/// la base que estaba activa antes de empezar.
///
/// **Efecto secundario importante:** tras el intercambio, esta función abre
/// la candidata y le aplica de verdad (y persiste) cualquier migración de
/// esquema pendiente — no es una verificación de solo lectura. Restaurar un
/// respaldo viejo deja el archivo restaurado en la versión de esquema
/// actual, no en la versión que tenía cuando se creó el respaldo.
pub fn restaurar_respaldo(ruta_candidata: &Path, ruta_activa: &Path) -> Result<(), RespaldoError> {
    let validacion = validar_respaldo(ruta_candidata)?;
    if !validacion.es_valido() {
        return Err(RespaldoError::ValidacionFallida(validacion));
    }

    let directorio = ruta_activa.parent().unwrap_or_else(|| Path::new("."));
    let ruta_temporal = directorio.join(".control_acceso_restauracion.tmp");
    let ruta_previa = directorio.join(".control_acceso_restauracion.previa");
    // A diferencia de otras limpiezas de este archivo, aquí sí se propaga:
    // si el sentinela de una restauración anterior existe pero no se puede
    // borrar (bloqueado, permisos), es mejor fallar aquí con un mensaje
    // claro que dejar que `fs::copy`/`fs::rename` fallen más adelante sobre
    // la misma ruta con un error genérico sin la pista real. `NotFound` (el
    // caso normal, sin sentinela previo) no cuenta como fallo.
    for ruta in [&ruta_temporal, &ruta_previa] {
        if ruta.exists() {
            fs::remove_file(ruta)?;
        }
    }

    // Paso 1: copiar la candidata a un temporal en el mismo directorio, sin
    // tocar todavía la base activa.
    fs::copy(ruta_candidata, &ruta_temporal)?;

    // Paso 2: intercambiar sin destruir inmediatamente la anterior — dos
    // renames en vez de una sobrescritura directa, para que en cualquier
    // punto intermedio quede algo recuperable.
    let habia_base_activa = ruta_activa.exists();
    if habia_base_activa {
        fs::rename(ruta_activa, &ruta_previa)?;
    }
    if let Err(error) = fs::rename(&ruta_temporal, ruta_activa) {
        if habia_base_activa {
            let _ = fs::rename(&ruta_previa, ruta_activa);
        }
        return Err(error.into());
    }

    // Paso 3: abrir la base ya intercambiada y aplicar sólo las migraciones
    // compatibles (initialize_database ya rechaza una versión futura).
    match abrir_y_verificar(ruta_activa) {
        Ok(()) => {
            let _ = fs::remove_file(&ruta_previa);
            Ok(())
        }
        Err(error) => {
            let limpio = fs::remove_file(ruta_activa).is_ok();
            let restaurado = !habia_base_activa || fs::rename(&ruta_previa, ruta_activa).is_ok();
            if limpio && restaurado {
                Err(error)
            } else {
                Err(RespaldoError::RollbackFallido {
                    error_original: Box::new(error),
                    ruta_activa: ruta_activa.to_path_buf(),
                    ruta_previa: habia_base_activa.then_some(ruta_previa),
                })
            }
        }
    }
}

fn abrir_y_verificar(ruta: &Path) -> Result<(), RespaldoError> {
    let connection = Connection::open(ruta)?;
    initialize_database(&connection)?;
    Ok(())
}

/// Abre `ruta` en modo sólo lectura (nunca crea ni modifica el archivo) y
/// corre las dos verificaciones que recomienda la documentación de SQLite:
/// `integrity_check` (estructura de páginas) y `foreign_key_check`
/// (relaciones entre tablas, que `integrity_check` no cubre).
pub fn validar_respaldo(ruta: &Path) -> Result<ResultadoValidacion, RespaldoError> {
    let connection = Connection::open_with_flags(ruta, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    let integridad: String =
        connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
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

    Ok(ResultadoValidacion::Valido { version_esquema })
}

/// Lista los respaldos ya publicados en `directorio_respaldos` (nunca los
/// `.partial`, que son un estado intermedio, no un respaldo real). No abre
/// SQLite — sólo lee el sistema de archivos y el nombre de cada archivo;
/// para saber si un respaldo sigue siendo válido hay que llamar
/// `validar_respaldo` aparte.
pub fn listar_respaldos(
    directorio_respaldos: &Path,
) -> Result<Vec<RespaldoResumen>, RespaldoError> {
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
        TipoRespaldo::PorFlag,
    ]
    .into_iter()
    .find(|tipo| coincide_sufijo(resto, tipo.sufijo()))?;

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

/// `resto` coincide con `sufijo` exacto, o con `sufijo` seguido de
/// `_<número>` (el sufijo de colisión que arma `ruta_disponible`, ej.
/// `_2`) — nunca con un prefijo arbitrario. Sin esto, un archivo renombrado
/// a mano como `..._automatico_no_borrar.db` coincidía con `Automatico`
/// (`"automatico_no_borrar".starts_with("automatico_")`) y quedaba expuesto
/// a que `aplicar_retencion` lo borrara igual que un respaldo real.
fn coincide_sufijo(resto: &str, sufijo: &str) -> bool {
    match resto.strip_prefix(sufijo) {
        Some("") => true,
        Some(cola) => cola
            .strip_prefix('_')
            .is_some_and(|numero| !numero.is_empty() && numero.bytes().all(|b| b.is_ascii_digit())),
        None => false,
    }
}

/// Conserva como máximo `limite` respaldos del `tipo` indicado — los más
/// recientes, según el mismo orden que ya usa `listar_respaldos` — y borra el
/// resto. Sólo actúa sobre el `tipo` recibido: nunca se le pasa
/// `TipoRespaldo::Manual`, `TipoRespaldo::PreRestauracion` ni
/// `TipoRespaldo::PorFlag`, así que esos no se tocan jamás desde esta función
/// — la política de retención documentada en `docs/pendientes.md` sólo cubre
/// respaldos automáticos y pre-migración. Un respaldo `PorFlag` es un punto
/// de recuperación deliberado (p. ej. antes de `--reset-root`); borrarlo solo
/// junto con los demás automáticos derrotaría su propósito.
pub fn aplicar_retencion(
    directorio_respaldos: &Path,
    tipo: TipoRespaldo,
    limite: usize,
) -> Result<Vec<PathBuf>, RespaldoError> {
    let mut candidatos: Vec<RespaldoResumen> = listar_respaldos(directorio_respaldos)?
        .into_iter()
        .filter(|respaldo| respaldo.tipo == tipo)
        .collect();
    let sobrantes = candidatos.split_off(limite.min(candidatos.len()));

    // Best-effort por archivo, no por lote: un respaldo bloqueado (en uso,
    // permisos) no debe impedir borrar el resto de los sobrantes. Los dos
    // únicos callers ya tratan el resultado completo como best-effort
    // (`let _ =`), así que abortar aquí con `?` sólo lograría borrar menos.
    let mut eliminados = Vec::with_capacity(sobrantes.len());
    for respaldo in sobrantes {
        if fs::remove_file(&respaldo.ruta).is_ok() {
            eliminados.push(respaldo.ruta);
        }
    }
    Ok(eliminados)
}

/// Nunca sobrescribe un archivo existente: si el nombre base ya está
/// ocupado, agrega un sufijo numérico creciente.
fn ruta_disponible(directorio: &Path, nombre_base: &str, extension: &str) -> PathBuf {
    let candidato = directorio.join(format!("{nombre_base}.{extension}"));
    if !candidato.exists() {
        return candidato;
    }
    (2..)
        .find_map(|n| {
            let candidato = directorio.join(format!("{nombre_base}_{n}.{extension}"));
            (!candidato.exists()).then_some(candidato)
        })
        .expect("el rango de sufijos numéricos es infinito")
}
