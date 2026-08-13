use chrono::NaiveDate;
use rusqlite::{Connection, Row, params};

use crate::database::error::DatabaseError;
use crate::models::tipo_ingreso::TipoIngreso;

const LIMITE_PREDETERMINADO: usize = 100;
const LIMITE_MAXIMO: usize = 500;

/// Lectura compuesta lista para presentar sin resolver la empresa por separado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContratistaResumen {
    pub id: i64,
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
        let texto = filtro
            .texto
            .as_deref()
            .map(str::trim)
            .filter(|texto| !texto.is_empty());
        let patron = texto.map(|texto| format!("%{texto}%"));
        let limite = filtro.limite.clamp(1, LIMITE_MAXIMO) as i64;
        let offset = filtro.offset as i64;

        let mut statement = self.connection.prepare(
            "
            SELECT
                c.id,
                c.cedula,
                c.nombre,
                e.nombre,
                c.tipo_ingreso,
                c.fecha_vencimiento_praind,
                c.es_personal_ruta,
                c.tiene_acceso
            FROM contratistas AS c
            INNER JOIN empresas AS e ON e.id = c.empresa_id
            WHERE ?1 IS NULL
               OR c.cedula LIKE ?1 COLLATE NOCASE
               OR c.nombre LIKE ?1 COLLATE NOCASE
               OR e.nombre LIKE ?1 COLLATE NOCASE
            ORDER BY c.nombre COLLATE NOCASE, c.id
            LIMIT ?2 OFFSET ?3
            ",
        )?;

        let resultados = statement
            .query_map(params![patron, limite, offset], convertir_fila)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(resultados)
    }
}

fn convertir_fila(row: &Row<'_>) -> rusqlite::Result<ContratistaResumen> {
    let tipo_texto: String = row.get(4)?;
    let tipo_ingreso = match tipo_texto.as_str() {
        "PRAIND" => TipoIngreso::Praind,
        "IN_HOUSE" => TipoIngreso::InHouse,
        "POR_CORREO" => TipoIngreso::PorCorreo,
        "SWAT" => TipoIngreso::Swat,
        _ => return Err(tipo_invalido(4, "tipo_ingreso")),
    };

    let fecha_texto: Option<String> = row.get(5)?;
    let fecha_vencimiento_praind = fecha_texto
        .map(|fecha| {
            NaiveDate::parse_from_str(&fecha, "%Y-%m-%d").map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })
        .transpose()?;

    Ok(ContratistaResumen {
        id: row.get(0)?,
        cedula: row.get(1)?,
        nombre: row.get(2)?,
        empresa_nombre: row.get(3)?,
        tipo_ingreso,
        fecha_vencimiento_praind,
        es_personal_ruta: row.get::<_, i64>(6)? != 0,
        tiene_acceso: row.get::<_, i64>(7)? != 0,
    })
}

fn tipo_invalido(indice: usize, nombre: &str) -> rusqlite::Error {
    rusqlite::Error::InvalidColumnType(indice, nombre.to_owned(), rusqlite::types::Type::Text)
}
