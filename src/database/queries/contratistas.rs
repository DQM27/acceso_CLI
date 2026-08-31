use chrono::{Duration, NaiveDate};
use rusqlite::{Connection, Row};

use crate::database::error::DatabaseError;
use crate::database::queries::{Igualdad, LIMITE_LISTADO_PREDETERMINADO as LIMITE_PREDETERMINADO};
use crate::database::search::BusquedaTexto;
use crate::domain::acceso::DIAS_ADVERTENCIA_PRAIND;
use crate::models::tipo_ingreso::TipoIngreso;

/// Tope propio de esta consulta, muy por encima de `LIMITE_LISTADO_MAXIMO`
/// (500, pensado para listados paginados como el de la TUI/`--comandos`) —
/// la GUI (Tauri) dejó de paginar contratistas: carga el universo completo
/// una sola vez y deja que AG Grid filtre/ordene del lado del cliente (ver
/// `desktop/src/pantallas/Contratistas.tsx`), pidiendo `limite: usize::MAX`
/// (`desktop/src-tauri/src/dto/contratistas.rs`). TUI/`--comandos` siguen
/// usando `LIMITE_PREDETERMINADO`/`LIMITE_LISTADO_MAXIMO` normales porque sí
/// paginan de verdad.
const LIMITE_LISTADO_MAXIMO_CARGA_COMPLETA: usize = 100_000;

/// Página de resultados: `items` respeta `limite`/`offset`, `total` es el conteo real sin
/// recortar por el filtro — así la UI puede mostrar "100 de 120" en vez de un tope silencioso.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PaginaContratistas {
    pub items: Vec<ContratistaResumen>,
    pub total: usize,
}

/// Lectura compuesta lista para presentar sin resolver la empresa por separado.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ContratistaResumen {
    pub id: i64,
    pub empresa_id: i64,
    pub cedula: String,
    pub nombre: String,
    pub empresa_nombre: String,
    pub tipo_ingreso: TipoIngreso,
    pub fecha_vencimiento_praind: Option<NaiveDate>,
    pub es_personal_ruta: bool,
    pub tiene_acceso: bool,
    /// Indica si existe un movimiento sin salida para mostrar el estado
    /// actual antes de intentar preparar un nuevo ingreso.
    pub tiene_ingreso_activo: bool,
}

/// Estado de vencimiento de PRAIND a filtrar. `hoy` viaja con la variante en
/// vez de como campo aparte de `FiltroContratistas` para no dejar un campo
/// "a veces relevante" — sólo importa cuando `praind` es `Some`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FiltroPraind {
    /// Vencido: tiene fecha registrada y ya pasó.
    Vencido { hoy: NaiveDate },
    /// Vence dentro de la misma ventana de advertencia que usa
    /// `domain::acceso::verificar_acceso` (30 días).
    ProximoAVencer { hoy: NaiveDate },
    /// Sin fecha de vencimiento registrada (contratistas que no requieren
    /// PRAIND, o a los que les falta el dato).
    SinFecha,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiltroContratistas {
    pub texto: Option<String>,
    pub empresa_id: Option<Igualdad<i64>>,
    /// `None` = todos los tipos; `Some(vec)` filtra a cualquiera de los
    /// listados (como máximo 4, la cantidad de variantes de `TipoIngreso`).
    pub tipos_incluidos: Option<Vec<TipoIngreso>>,
    pub praind: Option<FiltroPraind>,
    /// Sólo importa cuando `praind` es `Some`: si viene de `-praind:...`, el
    /// `WHERE` niega la condición completa de la variante en vez de
    /// intentar una variante "opuesta" (que no existe de forma unívoca para
    /// las 3 variantes de `FiltroPraind`).
    pub praind_negado: bool,
    pub personal_ruta: Option<bool>,
    pub tiene_acceso: Option<bool>,
    pub limite: usize,
    pub offset: usize,
}

impl Default for FiltroContratistas {
    fn default() -> Self {
        Self {
            texto: None,
            empresa_id: None,
            tipos_incluidos: None,
            praind: None,
            praind_negado: false,
            personal_ruta: None,
            tiene_acceso: None,
            limite: LIMITE_PREDETERMINADO,
            offset: 0,
        }
    }
}

pub trait ContratistasQuery {
    fn buscar(&self, filtro: &FiltroContratistas) -> Result<PaginaContratistas, DatabaseError>;
}

pub struct SqliteContratistasQuery<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteContratistasQuery<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

const CONTRATISTAS_FROM: &str = "
    FROM contratistas AS c
    INNER JOIN empresas AS e ON e.id = c.empresa_id
";

/// Nombre de parámetro (`:algo`) y valor bindeado. Usado sólo por los
/// constructores de `WHERE` dinámico de este archivo/`ingresos.rs`.
type ParametrosSql = Vec<(String, Box<dyn rusqlite::ToSql>)>;

/// Arma el `WHERE` sólo con las condiciones realmente activas (en vez de un
/// bloque fijo con flags `:x IS NULL OR col = :x` evaluados en cada fila) y
/// los parámetros que le corresponden a cada una. Con flags dinámicos,
/// SQLite no puede decidir en `prepare` qué rama aplica (depende del valor
/// del parámetro en runtime) y termina escaneando todas las filas sin usar
/// `idx_contratistas_empresa` aunque exista — confirmado con
/// `EXPLAIN QUERY PLAN` (`docs/hallazgos-buscador.md`). Omitir del todo la
/// condición cuando el filtro no aplica deja que el planificador vea
/// predicados concretos y elija índice, igual que ya hacen
/// `empresas.rs`/`usuarios.rs` para el modo de búsqueda.
fn construir_where(
    busqueda: &BusquedaTexto,
    filtro: &FiltroContratistas,
) -> (String, ParametrosSql) {
    let mut condiciones: Vec<String> = Vec::new();
    let mut parametros: ParametrosSql = Vec::new();

    // Sólo cédula y nombre — el buscador principal (`--comandos`: texto sin
    // `/`; TUI clásica: mismo filtro) no busca por empresa. Encontrar un
    // contratista tecleando el nombre de su empresa era una coincidencia
    // accidental del filtro compartido con `empresa:` de Historial, no un
    // criterio de búsqueda que el operador esperara — reportado en runtime
    // real.
    match busqueda.modo {
        1 => {
            condiciones.push(
                "(PLEGAR(c.cedula) LIKE PLEGAR(:patron) \
                  OR PLEGAR(c.nombre) LIKE PLEGAR(:patron))"
                    .into(),
            );
            parametros.push((":patron".into(), Box::new(busqueda.patron_like.clone())));
        }
        2 => {
            condiciones.push(
                "c.id IN (
                    SELECT rowid FROM contratistas_fts WHERE contratistas_fts MATCH :consulta_fts
                )"
                .into(),
            );
            parametros.push((
                ":consulta_fts".into(),
                Box::new(busqueda.consulta_fts.clone()),
            ));
        }
        _ => {}
    }

    if let Some(empresa_id) = filtro.empresa_id {
        condiciones.push(format!(
            "c.empresa_id {} :empresa_id",
            empresa_id.operador_sql()
        ));
        parametros.push((":empresa_id".into(), Box::new(*empresa_id.valor())));
    }

    if let Some(tipos) = &filtro.tipos_incluidos {
        let marcadores: Vec<String> = (0..tipos.len()).map(|i| format!(":tipo{i}")).collect();
        condiciones.push(format!("c.tipo_ingreso IN ({})", marcadores.join(", ")));
        for (marcador, tipo) in marcadores.into_iter().zip(tipos.iter()) {
            parametros.push((marcador, Box::new(tipo.as_str_sql())));
        }
    }

    let formato = |f: NaiveDate| f.format("%Y-%m-%d").to_string();
    if let Some(variante) = filtro.praind {
        // Cada rama ya incluye `IS NOT NULL`/`IS NULL` explícito, así que la
        // condición nunca evalúa a NULL (sólo TRUE/FALSE) y envolverla en
        // `NOT (...)` para `-praind:...` es una negación de 2 valores
        // correcta — no deja fuera a los contratistas sin fecha por la
        // lógica de 3 valores de SQL.
        let condicion = match variante {
            FiltroPraind::Vencido { hoy } => {
                parametros.push((":praind_hoy".into(), Box::new(formato(hoy))));
                "(c.fecha_vencimiento_praind IS NOT NULL \
                  AND c.fecha_vencimiento_praind < :praind_hoy \
                  AND (c.es_personal_ruta = 1 OR c.tipo_ingreso IN ('PRAIND', 'IN_HOUSE')))"
                    .to_string()
            }
            FiltroPraind::ProximoAVencer { hoy } => {
                parametros.push((":praind_hoy".into(), Box::new(formato(hoy))));
                parametros.push((
                    ":praind_limite".into(),
                    Box::new(formato(hoy + Duration::days(DIAS_ADVERTENCIA_PRAIND))),
                ));
                "(c.fecha_vencimiento_praind IS NOT NULL \
                  AND c.fecha_vencimiento_praind BETWEEN :praind_hoy AND :praind_limite \
                  AND (c.es_personal_ruta = 1 OR c.tipo_ingreso IN ('PRAIND', 'IN_HOUSE')))"
                    .to_string()
            }
            FiltroPraind::SinFecha => "c.fecha_vencimiento_praind IS NULL".to_string(),
        };
        condiciones.push(if filtro.praind_negado {
            format!("NOT ({condicion})")
        } else {
            condicion
        });
    }

    if let Some(personal_ruta) = filtro.personal_ruta {
        condiciones.push("c.es_personal_ruta = :personal_ruta".into());
        parametros.push((":personal_ruta".into(), Box::new(personal_ruta)));
    }
    if let Some(tiene_acceso) = filtro.tiene_acceso {
        condiciones.push("c.tiene_acceso = :tiene_acceso".into());
        parametros.push((":tiene_acceso".into(), Box::new(tiene_acceso)));
    }

    let where_sql = if condiciones.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", condiciones.join(" AND "))
    };
    (where_sql, parametros)
}

impl ContratistasQuery for SqliteContratistasQuery<'_> {
    fn buscar(&self, filtro: &FiltroContratistas) -> Result<PaginaContratistas, DatabaseError> {
        let busqueda = BusquedaTexto::preparar(filtro.texto.as_deref());
        let limite = filtro.limite.clamp(1, LIMITE_LISTADO_MAXIMO_CARGA_COMPLETA) as i64;
        let offset = filtro.offset as i64;
        let (where_sql, parametros) = construir_where(&busqueda, filtro);
        let params_comunes: Vec<(&str, &dyn rusqlite::ToSql)> = parametros
            .iter()
            .map(|(n, v)| (n.as_str(), v.as_ref()))
            .collect();

        let count_sql = format!("SELECT COUNT(*) {CONTRATISTAS_FROM} {where_sql}");
        let total: i64 =
            self.connection
                .query_row(&count_sql, params_comunes.as_slice(), |row| row.get(0))?;

        let select_sql = format!(
            "SELECT
                c.id, c.empresa_id, c.cedula, c.nombre, e.nombre, c.tipo_ingreso,
                c.fecha_vencimiento_praind, c.es_personal_ruta, c.tiene_acceso,
                EXISTS (
                    SELECT 1
                    FROM registro_ingresos AS r
                    WHERE r.contratista_id = c.id
                      AND r.fecha_hora_salida IS NULL
                )
             {CONTRATISTAS_FROM} {where_sql}
             ORDER BY CASE WHEN c.cedula = :texto_literal COLLATE NOCASE THEN 0 ELSE 1 END,
                      c.nombre COLLATE NOCASE, c.id
             LIMIT :limite OFFSET :offset"
        );
        let mut params_select = params_comunes;
        params_select.push((":texto_literal", &busqueda.texto_literal));
        params_select.push((":limite", &limite));
        params_select.push((":offset", &offset));

        let mut statement = self.connection.prepare(&select_sql)?;
        let items = statement
            .query_map(params_select.as_slice(), convertir_fila)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(PaginaContratistas {
            items,
            total: usize::try_from(total).unwrap_or(usize::MAX),
        })
    }
}

fn convertir_fila(row: &Row<'_>) -> rusqlite::Result<ContratistaResumen> {
    let tipo_texto: String = row.get(5)?;
    let Some(tipo_ingreso) = TipoIngreso::from_str_sql(&tipo_texto) else {
        return Err(tipo_invalido(5, "tipo_ingreso"));
    };

    let fecha_texto: Option<String> = row.get(6)?;
    let fecha_vencimiento_praind = fecha_texto
        .map(|fecha| {
            NaiveDate::parse_from_str(&fecha, "%Y-%m-%d").map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    6,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })
        .transpose()?;

    Ok(ContratistaResumen {
        id: row.get(0)?,
        empresa_id: row.get(1)?,
        cedula: row.get(2)?,
        nombre: row.get(3)?,
        empresa_nombre: row.get(4)?,
        tipo_ingreso,
        fecha_vencimiento_praind,
        es_personal_ruta: row.get::<_, i64>(7)? != 0,
        tiene_acceso: row.get::<_, i64>(8)? != 0,
        tiene_ingreso_activo: row.get::<_, i64>(9)? != 0,
    })
}

fn tipo_invalido(indice: usize, nombre: &str) -> rusqlite::Error {
    rusqlite::Error::InvalidColumnType(indice, nombre.to_owned(), rusqlite::types::Type::Text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::initialize_database;

    /// Regresión directa del hallazgo "WHERE con flags dinámicos impide usar
    /// índices" (`docs/hallazgos-buscador.md`): filtrar por `empresa_id` debe
    /// generar un predicado que el planificador pueda resolver con
    /// `idx_contratistas_empresa`, no un full scan.
    #[test]
    fn filtrar_por_empresa_usa_el_indice() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();

        let busqueda = BusquedaTexto::preparar(None);
        let filtro = FiltroContratistas {
            empresa_id: Some(Igualdad::Incluye(1)),
            ..FiltroContratistas::default()
        };
        let (where_sql, parametros) = construir_where(&busqueda, &filtro);
        let params: Vec<(&str, &dyn rusqlite::ToSql)> = parametros
            .iter()
            .map(|(n, v)| (n.as_str(), v.as_ref()))
            .collect();

        let sql = format!("EXPLAIN QUERY PLAN SELECT COUNT(*) {CONTRATISTAS_FROM} {where_sql}");
        let mut statement = connection.prepare(&sql).unwrap();
        let detalles: Vec<String> = statement
            .query_map(params.as_slice(), |row| row.get::<_, String>(3))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(
            detalles
                .iter()
                .any(|d| d.contains("idx_contratistas_empresa")),
            "{detalles:?}"
        );
    }

    /// Sin ningún filtro, el `WHERE` debe quedar vacío en vez de arrastrar
    /// condiciones siempre-verdaderas — evita el patrón que impedía usar
    /// índices incluso cuando no había nada realmente que filtrar.
    #[test]
    fn sin_filtros_el_where_queda_vacio() {
        let busqueda = BusquedaTexto::preparar(None);
        let (where_sql, parametros) = construir_where(&busqueda, &FiltroContratistas::default());
        assert_eq!(where_sql, "");
        assert!(parametros.is_empty());
    }

    /// `-empresa:...` (`docs/hallazgos-clave-valor.md`) debe armar `<>` en
    /// vez de `=`.
    #[test]
    fn negar_empresa_usa_distinto() {
        let busqueda = BusquedaTexto::preparar(None);
        let filtro = FiltroContratistas {
            empresa_id: Some(Igualdad::Excluye(1)),
            ..FiltroContratistas::default()
        };
        let (where_sql, _parametros) = construir_where(&busqueda, &filtro);
        assert!(
            where_sql.contains("c.empresa_id <> :empresa_id"),
            "{where_sql}"
        );
    }

    /// `-praind:...` niega la condición completa de la variante en vez de
    /// buscar una variante "opuesta" — cubre las 3 variantes de
    /// `FiltroPraind`, no sólo un caso.
    #[test]
    fn negar_praind_envuelve_la_condicion_en_not() {
        let busqueda = BusquedaTexto::preparar(None);
        let filtro = FiltroContratistas {
            praind: Some(FiltroPraind::SinFecha),
            praind_negado: true,
            ..FiltroContratistas::default()
        };
        let (where_sql, _parametros) = construir_where(&busqueda, &filtro);
        assert!(
            where_sql.contains("NOT (c.fecha_vencimiento_praind IS NULL)"),
            "{where_sql}"
        );
    }
}
