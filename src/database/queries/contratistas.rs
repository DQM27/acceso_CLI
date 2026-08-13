use chrono::NaiveDate;
use rusqlite::{Connection, Row};

use crate::database::error::DatabaseError;
use crate::database::search::BusquedaTexto;
use crate::models::tipo_ingreso::TipoIngreso;

const LIMITE_PREDETERMINADO: usize = 100;
const LIMITE_MAXIMO: usize = 500;

/// Lectura compuesta lista para presentar sin resolver la empresa por separado.
#[derive(Debug, Clone, PartialEq, Eq)]
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiltroContratistas {
    pub texto: Option<String>,
    pub limite: usize,
    pub offset: usize,
}

impl Default for FiltroContratistas {
    fn default() -> Self {
        Self {
            texto: None,
            limite: LIMITE_PREDETERMINADO,
            offset: 0,
        }
    }
}

pub trait ContratistasQuery {
    fn buscar(&self, filtro: &FiltroContratistas)
    -> Result<Vec<ContratistaResumen>, DatabaseError>;
}

pub struct SqliteContratistasQuery<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteContratistasQuery<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

impl ContratistasQuery for SqliteContratistasQuery<'_> {
    fn buscar(
        &self,
        filtro: &FiltroContratistas,
    ) -> Result<Vec<ContratistaResumen>, DatabaseError> {
        let busqueda = BusquedaTexto::preparar(filtro.texto.as_deref());
        let limite = filtro.limite.clamp(1, LIMITE_MAXIMO) as i64;
        let offset = filtro.offset as i64;

        let (sql, parametros): (&str, Vec<rusqlite::types::Value>) = match busqueda.modo {
            1 => (
                "
            SELECT
                c.id,
                c.empresa_id,
                c.cedula,
                c.nombre,
                e.nombre,
                c.tipo_ingreso,
                c.fecha_vencimiento_praind,
                c.es_personal_ruta,
                c.tiene_acceso
            FROM contratistas AS c
            INNER JOIN empresas AS e ON e.id = c.empresa_id
            WHERE c.cedula LIKE ?1 COLLATE NOCASE
               OR c.nombre LIKE ?1 COLLATE NOCASE
               OR e.nombre LIKE ?1 COLLATE NOCASE
            ORDER BY CASE WHEN c.cedula = ?2 COLLATE NOCASE THEN 0 ELSE 1 END,
                     c.nombre COLLATE NOCASE, c.id
            LIMIT ?3 OFFSET ?4
            ",
                vec![
                    busqueda.patron_like.into(),
                    busqueda.texto_literal.into(),
                    limite.into(),
                    offset.into(),
                ],
            ),
            2 => (
                "
            WITH coincidencias(id) AS (
                SELECT rowid FROM contratistas_fts WHERE contratistas_fts MATCH ?1
                UNION
                SELECT c.id
                FROM empresas_fts
                INNER JOIN contratistas AS c ON c.empresa_id = empresas_fts.rowid
                WHERE empresas_fts MATCH ?1
            )
            SELECT
                c.id, c.empresa_id, c.cedula, c.nombre, e.nombre, c.tipo_ingreso,
                c.fecha_vencimiento_praind, c.es_personal_ruta, c.tiene_acceso
            FROM coincidencias
            INNER JOIN contratistas AS c ON c.id = coincidencias.id
            INNER JOIN empresas AS e ON e.id = c.empresa_id
            ORDER BY CASE WHEN c.cedula = ?2 COLLATE NOCASE THEN 0 ELSE 1 END,
                     c.nombre COLLATE NOCASE, c.id
            LIMIT ?3 OFFSET ?4
            ",
                vec![
                    busqueda.consulta_fts.into(),
                    busqueda.texto_literal.into(),
                    limite.into(),
                    offset.into(),
                ],
            ),
            _ => (
                "
            SELECT
                c.id, c.empresa_id, c.cedula, c.nombre, e.nombre, c.tipo_ingreso,
                c.fecha_vencimiento_praind, c.es_personal_ruta, c.tiene_acceso
            FROM contratistas AS c
            INNER JOIN empresas AS e ON e.id = c.empresa_id
            ORDER BY c.nombre COLLATE NOCASE, c.id
            LIMIT ?1 OFFSET ?2
            ",
                vec![limite.into(), offset.into()],
            ),
        };
        let mut statement = self.connection.prepare(sql)?;

        let resultados = statement
            .query_map(rusqlite::params_from_iter(parametros), convertir_fila)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(resultados)
    }
}

fn convertir_fila(row: &Row<'_>) -> rusqlite::Result<ContratistaResumen> {
    let tipo_texto: String = row.get(5)?;
    let tipo_ingreso = match tipo_texto.as_str() {
        "PRAIND" => TipoIngreso::Praind,
        "IN_HOUSE" => TipoIngreso::InHouse,
        "POR_CORREO" => TipoIngreso::PorCorreo,
        "SWAT" => TipoIngreso::Swat,
        _ => return Err(tipo_invalido(5, "tipo_ingreso")),
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
    })
}

fn tipo_invalido(indice: usize, nombre: &str) -> rusqlite::Error {
    rusqlite::Error::InvalidColumnType(indice, nombre.to_owned(), rusqlite::types::Type::Text)
}
