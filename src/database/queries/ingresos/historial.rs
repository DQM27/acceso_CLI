//! Historial: paginado, ventanas más chicas que el resto de la app porque
//! `registro_ingresos` es append-only y crece sin límite.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, Row, Transaction, TransactionBehavior};

use crate::database::error::DatabaseError;
use crate::database::queries::Igualdad;
use crate::database::search::BusquedaTexto;
use crate::models::medio_ingreso::MedioIngreso;
use crate::models::registro_ingreso::{MotivoResultadoIngreso, ResultadoIngresoRegistrado};
use crate::models::tipo_ingreso::TipoIngreso;
use crate::tiempo::serializar_utc;

use super::{
    fecha_hora_desde_fila, medio_desde_fila, motivo_desde_fila, parsear_fecha_hora,
    resultado_desde_fila, tipo_desde_fila,
};

/// Más chico que `queries::LIMITE_LISTADO_PREDETERMINADO`/`_MAXIMO` (los que
/// comparten Contratistas/Empresas/Usuarios) a propósito: `registro_ingresos`
/// es append-only y crece sin límite, así que Historial pagina en ventanas
/// más chicas — no es una inconsistencia sin explicar.
const LIMITE_HISTORIAL_PREDETERMINADO: usize = 50;
const LIMITE_HISTORIAL_MAXIMO: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovimientoIngresoResumen {
    pub registro_id: i64,
    pub contratista_id: i64,
    pub cedula: String,
    pub contratista_nombre: String,
    pub empresa_nombre: String,
    pub tipo_ingreso: TipoIngreso,
    pub medio_ingreso: MedioIngreso,
    pub fecha_hora_ingreso: DateTime<Utc>,
    pub fecha_hora_salida: Option<DateTime<Utc>>,
    pub gafete_numero: Option<i64>,
    pub usuario_ingreso_nombre: String,
    pub usuario_salida_nombre: Option<String>,
    pub resultado_acceso: ResultadoIngresoRegistrado,
    pub motivo_resultado: Option<MotivoResultadoIngreso>,
    pub reglas_version: i64,
    /// Si la empresa del contratista estaba activa al momento de este
    /// ingreso — parte de la fotografía histórica, no un dato recalculado
    /// (`docs/pendientes.md`, hallazgo #7 de la auditoría de dominio).
    pub empresa_activa_snapshot: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoMovimiento {
    Todos,
    Activos,
    Cerrados,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiltroHistorial {
    /// Límite inferior inclusivo aplicado a `fecha_hora_ingreso`.
    pub desde: DateTime<Utc>,
    /// Límite superior exclusivo aplicado a `fecha_hora_ingreso`.
    pub hasta: DateTime<Utc>,
    pub texto_persona: Option<String>,
    pub empresa_id: Option<Igualdad<i64>>,
    /// `None` = todos los tipos. `Some(vec)` filtra a cualquiera de los tipos
    /// listados (como un `IN`); como máximo 4 (la cantidad de variantes de
    /// `TipoIngreso`), los excedentes se ignoran.
    pub tipos_incluidos: Option<Vec<TipoIngreso>>,
    pub gafete_numero: Option<Igualdad<i64>>,
    pub estado: EstadoMovimiento,
    /// Nombre (parcial, sin distinguir mayúsculas) del usuario que registró
    /// el ingreso.
    pub usuario_ingreso: Option<String>,
    /// Negar (`-ingreso:...`) sólo importa cuando `usuario_ingreso` es
    /// `Some`; la columna es `NOT NULL` así que la negación es una simple
    /// `NOT (...)`, sin casos especiales de NULL.
    pub usuario_ingreso_negado: bool,
    /// Nombre (parcial, sin distinguir mayúsculas) del usuario que registró
    /// la salida. Un movimiento sin salida nunca matchea en positivo.
    pub usuario_salida: Option<String>,
    /// Negar (`-salida:...`) sólo importa cuando `usuario_salida` es `Some`.
    /// A diferencia de `usuario_ingreso_negado`, la columna admite `NULL`
    /// (movimiento aún sin salida) — `construir_where_historial` incluye
    /// esas filas en el negado (no se cerró, así que no lo cerró esa
    /// persona) en vez de excluirlas por la lógica de 3 valores de SQL.
    pub usuario_salida_negado: bool,
    pub limite: usize,
    pub offset: usize,
    /// ID máximo visible en esta navegación. Excluye ingresos creados después de
    /// cargar la primera página para que las páginas no se desplacen.
    pub corte_id: Option<i64>,
}

impl FiltroHistorial {
    pub fn nuevo(desde: DateTime<Utc>, hasta: DateTime<Utc>) -> Self {
        Self {
            desde,
            hasta,
            texto_persona: None,
            empresa_id: None,
            tipos_incluidos: None,
            gafete_numero: None,
            estado: EstadoMovimiento::Todos,
            usuario_ingreso: None,
            usuario_ingreso_negado: false,
            usuario_salida: None,
            usuario_salida_negado: false,
            limite: LIMITE_HISTORIAL_PREDETERMINADO,
            offset: 0,
            corte_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaginaHistorial {
    pub items: Vec<MovimientoIngresoResumen>,
    pub total: usize,
    pub corte_id: i64,
}

const HISTORIAL_COLUMNAS: &str = "
    r.id, r.contratista_id, r.contratista_cedula, r.contratista_nombre,
    r.empresa_nombre, r.tipo_ingreso,
    r.medio_ingreso, r.fecha_hora_ingreso, r.fecha_hora_salida,
    r.gafete_numero, r.usuario_ingreso_nombre, r.usuario_salida_nombre,
    r.resultado_acceso, r.motivo_resultado, r.reglas_version,
    r.empresa_activa_snapshot
";

pub(super) fn buscar_historial(
    connection: &Connection,
    filtro: &FiltroHistorial,
) -> Result<PaginaHistorial, DatabaseError> {
    let transaction = Transaction::new_unchecked(connection, TransactionBehavior::Deferred)?;
    let busqueda = BusquedaTexto::preparar(filtro.texto_persona.as_deref());
    let limite = filtro.limite.clamp(1, LIMITE_HISTORIAL_MAXIMO) as i64;
    let offset = offset_sql(filtro.offset);
    let corte_id = match filtro.corte_id {
        Some(corte_id) => corte_id,
        None => transaction.query_row(
            "SELECT COALESCE(MAX(id), 0) FROM registro_ingresos",
            [],
            |row| row.get(0),
        )?,
    };

    let (where_sql, parametros) = construir_where_historial(&busqueda, filtro, corte_id);
    let params_comunes: Vec<(&str, &dyn rusqlite::ToSql)> = parametros
        .iter()
        .map(|(n, v)| (n.as_str(), v.as_ref()))
        .collect();

    let count_sql = format!("SELECT COUNT(*) FROM registro_ingresos AS r {where_sql}");
    let total: i64 =
        transaction.query_row(&count_sql, params_comunes.as_slice(), |row| row.get(0))?;

    let select_sql = format!(
        "SELECT {HISTORIAL_COLUMNAS} FROM registro_ingresos AS r {where_sql} \
         ORDER BY r.fecha_hora_ingreso DESC, r.id DESC LIMIT :limite OFFSET :offset"
    );
    let mut params_select = params_comunes;
    params_select.push((":limite", &limite));
    params_select.push((":offset", &offset));

    let mut statement = transaction.prepare(&select_sql)?;
    let items = statement
        .query_map(params_select.as_slice(), convertir_movimiento)?
        .collect::<Result<Vec<_>, _>>()?;
    drop(statement);
    transaction.commit()?;

    Ok(PaginaHistorial {
        items,
        total: usize::try_from(total).unwrap_or(usize::MAX),
        corte_id,
    })
}

/// Nombre de parámetro (`:algo`) y valor bindeado.
type ParametrosSql = Vec<(String, Box<dyn rusqlite::ToSql>)>;

/// Mismo criterio que `activos::construir_where_activos`. También resuelve
/// el hallazgo "Historial no encuentra por número de gafete en texto libre":
/// `registro_ingresos_fts` no indexa `gafete_numero` (sólo
/// cédula/nombre/empresa), así que el modo FTS/LIKE necesita la misma unión
/// explícita por gafete exacto que ya usa Activos.
fn construir_where_historial(
    busqueda: &BusquedaTexto,
    filtro: &FiltroHistorial,
    corte_id: i64,
) -> (String, ParametrosSql) {
    let mut condiciones: Vec<String> = vec![
        "r.fecha_hora_ingreso >= :desde".into(),
        "r.fecha_hora_ingreso < :hasta".into(),
        "r.id <= :corte_id".into(),
    ];
    let mut parametros: ParametrosSql = vec![
        (":desde".into(), Box::new(fecha_hora_a_texto(filtro.desde))),
        (":hasta".into(), Box::new(fecha_hora_a_texto(filtro.hasta))),
        (":corte_id".into(), Box::new(corte_id)),
    ];

    match busqueda.modo {
        1 => {
            let mut ramas = vec![
                "PLEGAR(r.contratista_cedula) LIKE PLEGAR(:patron)".to_string(),
                "PLEGAR(r.contratista_nombre) LIKE PLEGAR(:patron)".to_string(),
            ];
            parametros.push((":patron".into(), Box::new(busqueda.patron_like.clone())));
            if let Some(numero) = busqueda.numero_exacto {
                ramas.push("r.gafete_numero = :numero_exacto".to_string());
                parametros.push((":numero_exacto".into(), Box::new(numero)));
            }
            condiciones.push(format!("({})", ramas.join(" OR ")));
        }
        2 => {
            let mut sub =
                "SELECT rowid FROM registro_ingresos_fts WHERE registro_ingresos_fts MATCH :consulta_fts"
                    .to_string();
            parametros.push((
                ":consulta_fts".into(),
                Box::new(busqueda.consulta_fts.clone()),
            ));
            if let Some(numero) = busqueda.numero_exacto {
                sub.push_str(
                    " UNION SELECT id FROM registro_ingresos WHERE gafete_numero = :numero_exacto",
                );
                parametros.push((":numero_exacto".into(), Box::new(numero)));
            }
            condiciones.push(format!("r.id IN ({sub})"));
        }
        _ => {}
    }

    if let Some(empresa_id) = filtro.empresa_id {
        condiciones.push(format!(
            "r.empresa_id {} :empresa_id",
            empresa_id.operador_sql()
        ));
        parametros.push((":empresa_id".into(), Box::new(*empresa_id.valor())));
    }
    if let Some(tipos) = &filtro.tipos_incluidos {
        let marcadores: Vec<String> = (0..tipos.len()).map(|i| format!(":tipo{i}")).collect();
        condiciones.push(format!("r.tipo_ingreso IN ({})", marcadores.join(", ")));
        for (marcador, tipo) in marcadores.into_iter().zip(tipos.iter()) {
            parametros.push((marcador, Box::new(tipo.as_str_sql())));
        }
    }
    if let Some(gafete) = filtro.gafete_numero {
        condiciones.push(format!("r.gafete_numero {} :gafete", gafete.operador_sql()));
        parametros.push((":gafete".into(), Box::new(*gafete.valor())));
    }
    match filtro.estado {
        EstadoMovimiento::Todos => {}
        EstadoMovimiento::Activos => condiciones.push("r.fecha_hora_salida IS NULL".into()),
        EstadoMovimiento::Cerrados => condiciones.push("r.fecha_hora_salida IS NOT NULL".into()),
    }
    if let Some(usuario_ingreso) = &filtro.usuario_ingreso {
        parametros.push((
            ":usuario_ingreso".into(),
            Box::new(patron_like(usuario_ingreso)),
        ));
        // `usuario_ingreso_nombre` es `NOT NULL`: la comparación nunca da
        // NULL, así que `NOT (...)` alcanza para negar sin casos especiales.
        // `PLEGAR` pliega mayúsculas y diacríticos a la vez (igual que
        // `empresa:`/texto libre) — reemplaza a `COLLATE NOCASE`, que sólo
        // pliega mayúsculas ASCII y dejaba "salida:josé" sin encontrar
        // "José" o viceversa.
        condiciones.push(if filtro.usuario_ingreso_negado {
            "NOT (PLEGAR(r.usuario_ingreso_nombre) LIKE PLEGAR(:usuario_ingreso))".into()
        } else {
            "PLEGAR(r.usuario_ingreso_nombre) LIKE PLEGAR(:usuario_ingreso)".into()
        });
    }
    if let Some(usuario_salida) = &filtro.usuario_salida {
        parametros.push((
            ":usuario_salida".into(),
            Box::new(patron_like(usuario_salida)),
        ));
        condiciones.push(if filtro.usuario_salida_negado {
            // `usuario_salida_nombre` admite NULL (movimiento aún sin
            // salida) — se incluye explícitamente en la negación en vez de
            // dejar que la lógica de 3 valores de SQL lo excluya de ambos
            // lados (positivo y negado) por igual.
            "(r.usuario_salida_nombre IS NULL \
              OR PLEGAR(r.usuario_salida_nombre) NOT LIKE PLEGAR(:usuario_salida))"
                .into()
        } else {
            "PLEGAR(r.usuario_salida_nombre) LIKE PLEGAR(:usuario_salida)".into()
        });
    }

    (format!("WHERE {}", condiciones.join(" AND ")), parametros)
}

fn convertir_movimiento(row: &Row<'_>) -> rusqlite::Result<MovimientoIngresoResumen> {
    let motivo_resultado = motivo_desde_fila(row, 13)?;
    Ok(MovimientoIngresoResumen {
        registro_id: row.get(0)?,
        contratista_id: row.get(1)?,
        cedula: row.get(2)?,
        contratista_nombre: row.get(3)?,
        empresa_nombre: row.get(4)?,
        tipo_ingreso: tipo_desde_fila(row, 5)?,
        medio_ingreso: medio_desde_fila(row, 6)?,
        fecha_hora_ingreso: fecha_hora_desde_fila(row, 7)?,
        fecha_hora_salida: fecha_hora_opcional_desde_fila(row, 8)?,
        gafete_numero: row.get(9)?,
        usuario_ingreso_nombre: row.get(10)?,
        usuario_salida_nombre: row.get(11)?,
        resultado_acceso: resultado_desde_fila(row, 12, motivo_resultado)?,
        motivo_resultado,
        reglas_version: row.get(14)?,
        empresa_activa_snapshot: row.get::<_, i64>(15)? != 0,
    })
}

fn fecha_hora_opcional_desde_fila(
    row: &Row<'_>,
    indice: usize,
) -> rusqlite::Result<Option<DateTime<Utc>>> {
    let valor: Option<String> = row.get(indice)?;
    valor
        .map(|fecha| parsear_fecha_hora(&fecha, indice))
        .transpose()
}

fn patron_like(texto: &str) -> String {
    format!("%{}%", texto.trim())
}

fn fecha_hora_a_texto(fecha: DateTime<Utc>) -> String {
    serializar_utc(fecha)
}

fn offset_sql(offset: usize) -> i64 {
    i64::try_from(offset).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::initialize_database;

    fn plan(
        connection: &Connection,
        sql: &str,
        params: &[(&str, &dyn rusqlite::ToSql)],
    ) -> Vec<String> {
        let mut statement = connection
            .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
            .unwrap();
        statement
            .query_map(params, |row| row.get::<_, String>(3))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    /// Mismo hallazgo que Activos: el rango de fechas (siempre presente,
    /// nunca opcional) debe resolver con `idx_registro_ingresos_fecha_ingreso`.
    #[test]
    fn historial_rango_de_fechas_usa_el_indice() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();

        let busqueda = BusquedaTexto::preparar(None);
        let filtro = FiltroHistorial::nuevo(
            "2026-01-01T00:00:00Z".parse().unwrap(),
            "2026-02-01T00:00:00Z".parse().unwrap(),
        );
        let (where_sql, parametros) = construir_where_historial(&busqueda, &filtro, 0);
        let params: Vec<(&str, &dyn rusqlite::ToSql)> = parametros
            .iter()
            .map(|(n, v)| (n.as_str(), v.as_ref()))
            .collect();
        let detalles = plan(
            &connection,
            &format!("SELECT COUNT(*) FROM registro_ingresos AS r {where_sql}"),
            &params,
        );
        assert!(
            detalles
                .iter()
                .any(|d| d.contains("idx_registro_ingresos_fecha_ingreso")),
            "{detalles:?}"
        );
    }

    /// `-empresa:`/`-gafete:` deben armar `<>` en vez de `=`.
    #[test]
    fn negar_empresa_y_gafete_usa_distinto() {
        let busqueda = BusquedaTexto::preparar(None);

        let mut filtro = FiltroHistorial::nuevo(
            "2026-01-01T00:00:00Z".parse().unwrap(),
            "2026-02-01T00:00:00Z".parse().unwrap(),
        );
        filtro.empresa_id = Some(Igualdad::Excluye(1));
        filtro.gafete_numero = Some(Igualdad::Excluye(26));
        let (where_sql, _) = construir_where_historial(&busqueda, &filtro, 0);
        assert!(
            where_sql.contains("r.empresa_id <> :empresa_id"),
            "{where_sql}"
        );
        assert!(
            where_sql.contains("r.gafete_numero <> :gafete"),
            "{where_sql}"
        );
    }

    /// `-ingreso:`/`-salida:`: la columna de ingreso es `NOT NULL` así que
    /// basta con `NOT (...)`; la de salida admite `NULL` (movimiento aún
    /// abierto) y debe incluirse en la negación.
    #[test]
    fn negar_ingreso_y_salida() {
        let busqueda = BusquedaTexto::preparar(None);
        let mut filtro = FiltroHistorial::nuevo(
            "2026-01-01T00:00:00Z".parse().unwrap(),
            "2026-02-01T00:00:00Z".parse().unwrap(),
        );
        filtro.usuario_ingreso = Some("ana".into());
        filtro.usuario_ingreso_negado = true;
        filtro.usuario_salida = Some("ana".into());
        filtro.usuario_salida_negado = true;

        let (where_sql, _) = construir_where_historial(&busqueda, &filtro, 0);
        assert!(
            where_sql
                .contains("NOT (PLEGAR(r.usuario_ingreso_nombre) LIKE PLEGAR(:usuario_ingreso))"),
            "{where_sql}"
        );
        assert!(
            where_sql.contains("r.usuario_salida_nombre IS NULL"),
            "{where_sql}"
        );
        assert!(
            where_sql.contains("PLEGAR(r.usuario_salida_nombre) NOT LIKE PLEGAR(:usuario_salida)"),
            "{where_sql}"
        );
    }

    /// `ingreso:`/`salida:` deben plegar tildes igual que `empresa:`/texto
    /// libre vía la función SQL `PLEGAR` — antes usaban `COLLATE NOCASE`,
    /// que sólo pliega mayúsculas ASCII y no encuentra "José" con
    /// "salida:jose" ni viceversa (`PLEGAR` en sí ya está probada en
    /// `texto::tests`; aquí sólo se verifica que esta consulta la use).
    #[test]
    fn ingreso_y_salida_pliegan_tildes() {
        let busqueda = BusquedaTexto::preparar(None);
        let mut filtro = FiltroHistorial::nuevo(
            "2026-01-01T00:00:00Z".parse().unwrap(),
            "2026-02-01T00:00:00Z".parse().unwrap(),
        );
        filtro.usuario_ingreso = Some("José".into());
        filtro.usuario_salida = Some("María".into());
        let (where_sql, _) = construir_where_historial(&busqueda, &filtro, 0);
        assert!(
            where_sql.contains("PLEGAR(r.usuario_ingreso_nombre) LIKE PLEGAR(:usuario_ingreso)"),
            "{where_sql}"
        );
        assert!(
            where_sql.contains("PLEGAR(r.usuario_salida_nombre) LIKE PLEGAR(:usuario_salida)"),
            "{where_sql}"
        );
    }
}
