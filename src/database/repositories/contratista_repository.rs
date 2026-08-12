use chrono::NaiveDate;
use rusqlite::{Connection, Row, params};

use crate::database::error::DatabaseError;
use crate::models::contratista::Contratista;
use crate::models::tipo_ingreso::TipoIngreso;

pub trait ContratistaRepository {
    fn crear(&self, contratista: &Contratista) -> Result<i64, DatabaseError>;

    fn buscar_por_cedula(&self, cedula: &str) -> Result<Option<Contratista>, DatabaseError>;

    fn buscar_por_id(&self, id: i64) -> Result<Option<Contratista>, DatabaseError>;

    fn actualizar(&self, contratista: &Contratista) -> Result<(), DatabaseError>;

    fn listar(&self) -> Result<Vec<Contratista>, DatabaseError>;
}

pub struct SqliteContratistaRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteContratistaRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

fn convertir_fila(row: &Row) -> rusqlite::Result<Contratista> {
    let tipo_ingreso_texto: String = row.get(4)?;

    let tipo_ingreso = match tipo_ingreso_texto.as_str() {
        "PRAIND" => TipoIngreso::Praind,
        "IN_HOUSE" => TipoIngreso::InHouse,
        "POR_CORREO" => TipoIngreso::PorCorreo,
        "SWAT" => TipoIngreso::Swat,

        _ => {
            return Err(rusqlite::Error::InvalidColumnType(
                4,
                "tipo_ingreso".to_string(),
                rusqlite::types::Type::Text,
            ));
        }
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

    Ok(Contratista {
        id: row.get(0)?,
        cedula: row.get(1)?,
        nombre: row.get(2)?,
        empresa_id: row.get(3)?,
        tipo_ingreso,
        fecha_vencimiento_praind,
        es_personal_ruta: row.get::<_, i64>(6)? != 0,
        tiene_acceso: row.get::<_, i64>(7)? != 0,
    })
}

impl<'a> ContratistaRepository for SqliteContratistaRepository<'a> {
    fn crear(&self, contratista: &Contratista) -> Result<i64, DatabaseError> {
        let fecha_vencimiento = contratista
            .fecha_vencimiento_praind
            .map(|fecha| fecha.format("%Y-%m-%d").to_string());

        let tipo_ingreso = match contratista.tipo_ingreso {
            TipoIngreso::Praind => "PRAIND",
            TipoIngreso::InHouse => "IN_HOUSE",
            TipoIngreso::PorCorreo => "POR_CORREO",
            TipoIngreso::Swat => "SWAT",
        };

        self.connection.execute(
            "
            INSERT INTO contratistas (
                cedula,
                nombre,
                empresa_id,
                tipo_ingreso,
                fecha_vencimiento_praind,
                es_personal_ruta,
                tiene_acceso
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ",
            params![
                contratista.cedula,
                contratista.nombre,
                contratista.empresa_id,
                tipo_ingreso,
                fecha_vencimiento,
                contratista.es_personal_ruta as i64,
                contratista.tiene_acceso as i64,
            ],
        )?;

        Ok(self.connection.last_insert_rowid())
    }

    fn buscar_por_cedula(&self, cedula: &str) -> Result<Option<Contratista>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                id,
                cedula,
                nombre,
                empresa_id,
                tipo_ingreso,
                fecha_vencimiento_praind,
                es_personal_ruta,
                tiene_acceso
            FROM contratistas
            WHERE cedula = ?1
            ",
        )?;

        match statement.query_row(params![cedula], convertir_fila) {
            Ok(contratista) => Ok(Some(contratista)),

            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),

            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn buscar_por_id(&self, id: i64) -> Result<Option<Contratista>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                id,
                cedula,
                nombre,
                empresa_id,
                tipo_ingreso,
                fecha_vencimiento_praind,
                es_personal_ruta,
                tiene_acceso
            FROM contratistas
            WHERE id = ?1
            ",
        )?;

        match statement.query_row(params![id], convertir_fila) {
            Ok(contratista) => Ok(Some(contratista)),

            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),

            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn actualizar(&self, contratista: &Contratista) -> Result<(), DatabaseError> {
        let fecha_vencimiento = contratista
            .fecha_vencimiento_praind
            .map(|fecha| fecha.format("%Y-%m-%d").to_string());

        let tipo_ingreso = match contratista.tipo_ingreso {
            TipoIngreso::Praind => "PRAIND",
            TipoIngreso::InHouse => "IN_HOUSE",
            TipoIngreso::PorCorreo => "POR_CORREO",
            TipoIngreso::Swat => "SWAT",
        };

        self.connection.execute(
            "
            UPDATE contratistas
            SET
                cedula = ?1,
                nombre = ?2,
                empresa_id = ?3,
                tipo_ingreso = ?4,
                fecha_vencimiento_praind = ?5,
                es_personal_ruta = ?6,
                tiene_acceso = ?7
            WHERE id = ?8
            ",
            params![
                contratista.cedula,
                contratista.nombre,
                contratista.empresa_id,
                tipo_ingreso,
                fecha_vencimiento,
                contratista.es_personal_ruta as i64,
                contratista.tiene_acceso as i64,
                contratista.id,
            ],
        )?;

        Ok(())
    }

    fn listar(&self) -> Result<Vec<Contratista>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                id,
                cedula,
                nombre,
                empresa_id,
                tipo_ingreso,
                fecha_vencimiento_praind,
                es_personal_ruta,
                tiene_acceso
            FROM contratistas
            ORDER BY nombre
            ",
        )?;

        let contratistas = statement
            .query_map([], convertir_fila)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(contratistas)
    }
}
