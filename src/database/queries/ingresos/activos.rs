//! Ingresos Activos: sin paginación, con un tope de seguridad — a diferencia
//! de Historial, aquí no hay "página siguiente", sólo la lista completa de
//! personas adentro ahora mismo.

use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{Connection, Row};

use crate::database::error::DatabaseError;
use crate::database::queries::Igualdad;
use crate::database::search::BusquedaTexto;
use crate::models::medio_ingreso::MedioIngreso;
use crate::models::registro_ingreso::ResultadoIngresoRegistrado;
use crate::models::tipo_ingreso::TipoIngreso;

use super::{
    fecha_hora_desde_fila, medio_desde_fila, motivo_desde_fila, resultado_desde_fila,
    tipo_desde_fila,
};

/// Tope de seguridad para Ingresos Activos, la única consulta de la app que
/// antes no tenía ninguno. No hay paginación en esa pantalla (a diferencia de
/// Historial) — este límite es sólo para no cargar sin fin si algún día el
/// número de ingresos sin cerrar crece de forma anómala.
const LIMITE_ACTIVOS_PREDETERMINADO: usize = 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngresoActivoLectura {
    pub registro_id: i64,
    pub contratista_id: i64,
    pub empresa_id: i64,
    pub cedula: String,
    pub contratista_nombre: String,
    pub empresa_nombre: String,
    pub tipo_ingreso: TipoIngreso,
    pub medio_ingreso: MedioIngreso,
    pub fecha_hora_ingreso: DateTime<Utc>,
    pub gafete_numero: Option<i64>,
    pub usuario_ingreso_nombre: String,
    pub fecha_vencimiento_praind: Option<NaiveDate>,
    pub es_personal_ruta: bool,
    pub tiene_acceso: bool,
    pub empresa_activa: bool,
    pub resultado_registrado: ResultadoIngresoRegistrado,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiltroIngresosActivos {
    pub texto: Option<String>,
    pub empresa_id: Option<Igualdad<i64>>,
    /// `None` = todos los tipos; `Some(vec)` filtra a cualquiera de los
    /// listados (como máximo 4, la cantidad de variantes de `TipoIngreso`).
    pub tipos_incluidos: Option<Vec<TipoIngreso>>,
    pub gafete_numero: Option<Igualdad<i64>>,
    pub medio_ingreso: Option<MedioIngreso>,
    pub limite: usize,
}

impl Default for FiltroIngresosActivos {
    fn default() -> Self {
        Self {
            texto: None,
            empresa_id: None,
            tipos_incluidos: None,
            gafete_numero: None,
            medio_ingreso: None,
            limite: LIMITE_ACTIVOS_PREDETERMINADO,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListaIngresosActivosLectura {
    pub items: Vec<IngresoActivoLectura>,
    pub total: usize,
}

pub(super) fn listar_activos(
    connection: &Connection,
    filtro: &FiltroIngresosActivos,
) -> Result<ListaIngresosActivosLectura, DatabaseError> {
    let busqueda = BusquedaTexto::preparar(filtro.texto.as_deref());
    // Total real de personas dentro, sin aplicar el filtro — decisión
    // deliberada (ver docs/pendientes.md): la UI usa "N de M dentro" con
    // M = total sin filtrar, no M = coincidencias.
    let total: i64 = connection.query_row(
        "SELECT COUNT(*) FROM registro_ingresos WHERE fecha_hora_salida IS NULL",
        [],
        |row| row.get(0),
    )?;

    let limite =
        i64::try_from(filtro.limite.clamp(1, LIMITE_ACTIVOS_PREDETERMINADO)).unwrap_or(i64::MAX);
    let (where_sql, parametros) = construir_where_activos(&busqueda, filtro);
    let mut params: Vec<(&str, &dyn rusqlite::ToSql)> = parametros
        .iter()
        .map(|(n, v)| (n.as_str(), v.as_ref()))
        .collect();
    params.push((":limite", &limite));

    let select_sql = format!(
        "SELECT
            r.id, r.contratista_id, r.empresa_id, r.contratista_cedula,
            r.contratista_nombre, r.empresa_nombre,
            r.tipo_ingreso, r.medio_ingreso, r.fecha_hora_ingreso, r.gafete_numero,
            r.usuario_ingreso_nombre, c.fecha_vencimiento_praind,
            c.es_personal_ruta, c.tiene_acceso, e.activo,
            r.resultado_acceso, r.motivo_resultado
         FROM registro_ingresos AS r
         INNER JOIN contratistas AS c ON c.id = r.contratista_id
         INNER JOIN empresas AS e ON e.id = c.empresa_id
         {where_sql}
         ORDER BY r.fecha_hora_ingreso DESC, r.id DESC
         LIMIT :limite"
    );
    let mut statement = connection.prepare(&select_sql)?;
    let items = statement
        .query_map(params.as_slice(), convertir_activo)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ListaIngresosActivosLectura {
        items,
        total: usize::try_from(total).unwrap_or(usize::MAX),
    })
}

/// Nombre de parámetro (`:algo`) y valor bindeado.
type ParametrosSql = Vec<(String, Box<dyn rusqlite::ToSql>)>;

/// Igual que `contratistas::construir_where`: arma el `WHERE` sólo con las
/// condiciones realmente activas, sin flags `:x IS NULL OR col = :x`
/// evaluados en cada fila — eso es lo que le impedía a `SQLite` usar
/// `idx_registro_ingresos_empresa` aunque existiera (confirmado con
/// `EXPLAIN QUERY PLAN`).
fn construir_where_activos(
    busqueda: &BusquedaTexto,
    filtro: &FiltroIngresosActivos,
) -> (String, ParametrosSql) {
    let mut condiciones: Vec<String> = vec!["r.fecha_hora_salida IS NULL".into()];
    let mut parametros: ParametrosSql = Vec::new();

    match busqueda.modo {
        1 => {
            let mut ramas = vec![
                "PLEGAR(r.contratista_cedula) LIKE PLEGAR(:patron)".to_string(),
                "PLEGAR(r.contratista_nombre) LIKE PLEGAR(:patron)".to_string(),
                "PLEGAR(r.empresa_nombre) LIKE PLEGAR(:patron)".to_string(),
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
                    " UNION SELECT id FROM registro_ingresos \
                      WHERE gafete_numero = :numero_exacto AND fecha_hora_salida IS NULL",
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
    if let Some(medio) = filtro.medio_ingreso {
        condiciones.push("r.medio_ingreso = :medio".into());
        parametros.push((":medio".into(), Box::new(medio_a_texto(medio))));
    }

    (format!("WHERE {}", condiciones.join(" AND ")), parametros)
}

fn convertir_activo(row: &Row<'_>) -> rusqlite::Result<IngresoActivoLectura> {
    let motivo_resultado = motivo_desde_fila(row, 16)?;
    Ok(IngresoActivoLectura {
        registro_id: row.get(0)?,
        contratista_id: row.get(1)?,
        empresa_id: row.get(2)?,
        cedula: row.get(3)?,
        contratista_nombre: row.get(4)?,
        empresa_nombre: row.get(5)?,
        tipo_ingreso: tipo_desde_fila(row, 6)?,
        medio_ingreso: medio_desde_fila(row, 7)?,
        fecha_hora_ingreso: fecha_hora_desde_fila(row, 8)?,
        gafete_numero: row.get(9)?,
        usuario_ingreso_nombre: row.get(10)?,
        fecha_vencimiento_praind: fecha_desde_fila(row, 11)?,
        es_personal_ruta: row.get::<_, i64>(12)? != 0,
        tiene_acceso: row.get::<_, i64>(13)? != 0,
        empresa_activa: row.get::<_, i64>(14)? != 0,
        resultado_registrado: resultado_desde_fila(row, 15, motivo_resultado)?,
    })
}

fn fecha_desde_fila(row: &Row<'_>, indice: usize) -> rusqlite::Result<Option<NaiveDate>> {
    let valor: Option<String> = row.get(indice)?;
    valor.map(|fecha| parsear_fecha(&fecha, indice)).transpose()
}

fn parsear_fecha(valor: &str, indice: usize) -> rusqlite::Result<NaiveDate> {
    NaiveDate::parse_from_str(valor, "%Y-%m-%d").map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            indice,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

fn medio_a_texto(medio: MedioIngreso) -> &'static str {
    match medio {
        MedioIngreso::Caminando => "CAMINANDO",
        MedioIngreso::Vehiculo => "VEHICULO",
    }
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

    /// Regresión directa del hallazgo "WHERE con flags dinámicos impide usar
    /// índices" para Activos.
    #[test]
    fn activos_filtrar_por_empresa_usa_el_indice() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();

        let busqueda = BusquedaTexto::preparar(None);
        let filtro = FiltroIngresosActivos {
            empresa_id: Some(Igualdad::Incluye(1)),
            ..FiltroIngresosActivos::default()
        };
        let (where_sql, parametros) = construir_where_activos(&busqueda, &filtro);
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
                .any(|d| d.contains("idx_registro_ingresos_empresa")),
            "{detalles:?}"
        );
    }

    /// `-empresa:`/`-gafete:` deben armar `<>` en vez de `=`.
    #[test]
    fn negar_empresa_y_gafete_usa_distinto() {
        let busqueda = BusquedaTexto::preparar(None);

        let filtro = FiltroIngresosActivos {
            empresa_id: Some(Igualdad::Excluye(1)),
            gafete_numero: Some(Igualdad::Excluye(26)),
            ..FiltroIngresosActivos::default()
        };
        let (where_sql, _) = construir_where_activos(&busqueda, &filtro);
        assert!(
            where_sql.contains("r.empresa_id <> :empresa_id"),
            "{where_sql}"
        );
        assert!(
            where_sql.contains("r.gafete_numero <> :gafete"),
            "{where_sql}"
        );
    }
}
