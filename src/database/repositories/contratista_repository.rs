use chrono::NaiveDate;
use rusqlite::{Connection, Row, params};

use crate::database::cola_salida;
use crate::database::error::DatabaseError;
use crate::database::identificador::generar_uuid_v4;
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

    let Some(tipo_ingreso) = TipoIngreso::from_str_sql(&tipo_ingreso_texto) else {
        return Err(rusqlite::Error::InvalidColumnType(
            4,
            "tipo_ingreso".to_string(),
            rusqlite::types::Type::Text,
        ));
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

    Ok(Contratista::reconstruir(
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        tipo_ingreso,
        fecha_vencimiento_praind,
        row.get::<_, i64>(6)? != 0,
        row.get::<_, i64>(7)? != 0,
        row.get::<_, i64>(8)? != 0,
    ))
}

impl ContratistaRepository for SqliteContratistaRepository<'_> {
    fn crear(&self, contratista: &Contratista) -> Result<i64, DatabaseError> {
        let fecha_vencimiento = contratista
            .fecha_vencimiento_praind
            .map(|fecha| fecha.format("%Y-%m-%d").to_string());

        let tipo_ingreso = contratista.tipo_ingreso.as_str_sql();
        let uuid = generar_uuid_v4();

        self.connection.execute(
            "
            INSERT INTO contratistas (
                cedula,
                nombre,
                empresa_id,
                tipo_ingreso,
                fecha_vencimiento_praind,
                es_personal_ruta,
                tiene_acceso,
                uuid
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ",
            params![
                contratista.cedula,
                contratista.nombre,
                contratista.empresa_id,
                tipo_ingreso,
                fecha_vencimiento,
                i64::from(contratista.es_personal_ruta),
                i64::from(contratista.tiene_acceso),
                uuid,
            ],
        )?;

        // Capturado antes de encolar: `last_insert_rowid()` refleja el
        // último INSERT de la conexión, y encolar hace el suyo propio.
        let id = self.connection.last_insert_rowid();
        cola_salida::encolar(self.connection, "contratista", &uuid, "crear")?;

        Ok(id)
    }

    fn buscar_por_cedula(&self, cedula: &str) -> Result<Option<Contratista>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                c.id,
                c.cedula,
                c.nombre,
                c.empresa_id,
                c.tipo_ingreso,
                c.fecha_vencimiento_praind,
                c.es_personal_ruta,
                c.tiene_acceso,
                e.activo
            FROM contratistas c
            JOIN empresas e ON e.id = c.empresa_id
            WHERE c.cedula = ?1
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
                c.id,
                c.cedula,
                c.nombre,
                c.empresa_id,
                c.tipo_ingreso,
                c.fecha_vencimiento_praind,
                c.es_personal_ruta,
                c.tiene_acceso,
                e.activo
            FROM contratistas c
            JOIN empresas e ON e.id = c.empresa_id
            WHERE c.id = ?1
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

        let tipo_ingreso = contratista.tipo_ingreso.as_str_sql();

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
                i64::from(contratista.es_personal_ruta),
                i64::from(contratista.tiene_acceso),
                contratista.id,
            ],
        )?;

        let uuid: Option<String> = self.connection.query_row(
            "SELECT uuid FROM contratistas WHERE id = ?1",
            params![contratista.id],
            |row| row.get(0),
        )?;
        if let Some(uuid) = uuid {
            cola_salida::encolar(self.connection, "contratista", &uuid, "actualizar")?;
        }

        Ok(())
    }

    fn listar(&self) -> Result<Vec<Contratista>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                c.id,
                c.cedula,
                c.nombre,
                c.empresa_id,
                c.tipo_ingreso,
                c.fecha_vencimiento_praind,
                c.es_personal_ruta,
                c.tiene_acceso,
                e.activo
            FROM contratistas c
            JOIN empresas e ON e.id = c.empresa_id
            ORDER BY c.nombre
            ",
        )?;

        let contratistas = statement
            .query_map([], convertir_fila)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(contratistas)
    }
}
