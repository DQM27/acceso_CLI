//! Lectura del catálogo de gafetes (`docs/plan-gafetes.md`). El catálogo es
//! chico (decenas de filas, uno por gafete físico) — se trae entero, sin
//! paginar, mismo criterio que `queries::ingresos::activos` para Ingresos
//! Activos.

use rusqlite::{Connection, Row, params_from_iter};

use crate::database::error::DatabaseError;
use crate::database::queries::Igualdad;
use crate::models::gafete::EstadoGafete;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct GafeteResumen {
    pub id: i64,
    pub numero: i64,
    pub estado: EstadoGafete,
    pub contratista_deudor_id: Option<i64>,
    pub contratista_deudor_nombre: Option<String>,
    /// Fecha del incidente `PERDIDO` más reciente — sólo tiene sentido
    /// mostrarla cuando `estado == Perdido` (mientras el gafete esté
    /// disponible o de baja, el incidente que la generó ya fue resuelto).
    pub fecha_marcado_perdido: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FiltroGafetes {
    pub numero: Option<i64>,
    pub estado: Option<Igualdad<EstadoGafete>>,
}

pub trait GafetesQuery {
    fn buscar(&self, filtro: &FiltroGafetes) -> Result<Vec<GafeteResumen>, DatabaseError>;
}

pub struct SqliteGafetesQuery<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteGafetesQuery<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

impl GafetesQuery for SqliteGafetesQuery<'_> {
    fn buscar(&self, filtro: &FiltroGafetes) -> Result<Vec<GafeteResumen>, DatabaseError> {
        let mut condiciones: Vec<String> = Vec::new();
        let mut parametros: Vec<rusqlite::types::Value> = Vec::new();

        if let Some(numero) = filtro.numero {
            condiciones.push("g.numero = ?".into());
            parametros.push(numero.into());
        }
        if let Some(estado) = filtro.estado {
            condiciones.push(format!("g.estado {} ?", estado.operador_sql()));
            parametros.push(estado.valor().as_str_sql().to_string().into());
        }
        let where_sql = if condiciones.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", condiciones.join(" AND "))
        };

        let sql = format!(
            "SELECT
                g.id, g.numero, g.estado, g.contratista_deudor_id, c.nombre,
                (SELECT gi.fecha_hora FROM gafetes_incidentes gi
                 WHERE gi.gafete_id = g.id AND gi.tipo = 'PERDIDO'
                 ORDER BY gi.id DESC LIMIT 1)
             FROM gafetes g
             LEFT JOIN contratistas c ON c.id = g.contratista_deudor_id
             {where_sql}
             ORDER BY g.numero"
        );
        let mut statement = self.connection.prepare(&sql)?;
        Ok(statement
            .query_map(params_from_iter(parametros), convertir_fila)?
            .collect::<Result<Vec<_>, _>>()?)
    }
}

fn convertir_fila(row: &Row<'_>) -> rusqlite::Result<GafeteResumen> {
    let estado_texto: String = row.get(2)?;
    let Some(estado) = EstadoGafete::from_str_sql(&estado_texto) else {
        return Err(rusqlite::Error::InvalidColumnType(
            2,
            "estado".to_string(),
            rusqlite::types::Type::Text,
        ));
    };

    Ok(GafeteResumen {
        id: row.get(0)?,
        numero: row.get(1)?,
        estado,
        contratista_deudor_id: row.get(3)?,
        contratista_deudor_nombre: row.get(4)?,
        fecha_marcado_perdido: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::initialize_database;

    #[test]
    fn sin_filtros_trae_todo_ordenado_por_numero() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO gafetes (numero, estado) VALUES (2, 'DISPONIBLE'), (1, 'DISPONIBLE')",
            )
            .unwrap();

        let query = SqliteGafetesQuery::new(&connection);
        let items = query.buscar(&FiltroGafetes::default()).unwrap();

        assert_eq!(
            items.iter().map(|g| g.numero).collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    #[test]
    fn filtra_por_estado_negado() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO gafetes (numero, estado) VALUES (1, 'DISPONIBLE'), (2, 'DE_BAJA')",
            )
            .unwrap();

        let query = SqliteGafetesQuery::new(&connection);
        let filtro = FiltroGafetes {
            estado: Some(Igualdad::Excluye(EstadoGafete::DeBaja)),
            ..FiltroGafetes::default()
        };
        let items = query.buscar(&filtro).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].numero, 1);
    }
}
