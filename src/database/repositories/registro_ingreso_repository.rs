use chrono::NaiveDateTime;
use rusqlite::{Connection, Row, params};

use crate::database::error::DatabaseError;
use crate::models::medio_ingreso::MedioIngreso;
use crate::models::registro_ingreso::RegistroIngreso;
use crate::models::tipo_ingreso::TipoIngreso;

pub trait RegistroIngresoRepository {
    fn crear(&self, registro: &RegistroIngreso) -> Result<i64, DatabaseError>;

    fn buscar_por_id(&self, id: i64) -> Result<Option<RegistroIngreso>, DatabaseError>;

    fn buscar_ingreso_activo(
        &self,
        contratista_id: i64,
    ) -> Result<Option<RegistroIngreso>, DatabaseError>;

    /// Busca quién tiene actualmente asignado un gafete.
    ///
    /// Solo considera ingresos activos, es decir,
    /// registros que todavía no tienen fecha de salida.
    fn buscar_ingreso_activo_por_gafete(
        &self,
        gafete_numero: i64,
    ) -> Result<Option<RegistroIngreso>, DatabaseError>;

    fn registrar_salida(
        &self,
        id: i64,
        fecha_hora_salida: NaiveDateTime,
        usuario_salida_id: i64,
    ) -> Result<(), DatabaseError>;

    fn listar(&self) -> Result<Vec<RegistroIngreso>, DatabaseError>;
}

pub struct SqliteRegistroIngresoRepository<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteRegistroIngresoRepository<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

fn convertir_fila(row: &Row) -> rusqlite::Result<RegistroIngreso> {
    let fecha_hora_ingreso_texto: String = row.get(3)?;

    let fecha_hora_ingreso = NaiveDateTime::parse_from_str(
        &fecha_hora_ingreso_texto,
        "%Y-%m-%d %H:%M:%S",
    )
    .map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(error))
    })?;

    let medio_ingreso_texto: String = row.get(4)?;

    let medio_ingreso = match medio_ingreso_texto.as_str() {
        "CAMINANDO" => MedioIngreso::Caminando,
        "VEHICULO" => MedioIngreso::Vehiculo,

        _ => {
            return Err(rusqlite::Error::InvalidColumnType(
                4,
                "medio_ingreso".to_string(),
                rusqlite::types::Type::Text,
            ));
        }
    };

    let tipo_ingreso_texto: String = row.get(5)?;

    let tipo_ingreso = match tipo_ingreso_texto.as_str() {
        "PRAIND" => TipoIngreso::Praind,
        "IN_HOUSE" => TipoIngreso::InHouse,
        "POR_CORREO" => TipoIngreso::PorCorreo,
        "SWAT" => TipoIngreso::Swat,

        _ => {
            return Err(rusqlite::Error::InvalidColumnType(
                5,
                "tipo_ingreso".to_string(),
                rusqlite::types::Type::Text,
            ));
        }
    };

    // El gafete es opcional.
    //
    // NULL = S/G
    // número = gafete asignado
    let gafete_numero: Option<i64> = row.get(6)?;

    let fecha_hora_salida_texto: Option<String> = row.get(8)?;

    let fecha_hora_salida = fecha_hora_salida_texto
        .map(|fecha| {
            NaiveDateTime::parse_from_str(&fecha, "%Y-%m-%d %H:%M:%S").map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    8,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })
        .transpose()?;

    Ok(RegistroIngreso {
        id: row.get(0)?,
        contratista_id: row.get(1)?,
        empresa_id: row.get(2)?,
        fecha_hora_ingreso,
        medio_ingreso,
        tipo_ingreso,
        gafete_numero,
        usuario_ingreso_id: row.get(7)?,
        fecha_hora_salida,
        usuario_salida_id: row.get(9)?,
    })
}

impl<'a> RegistroIngresoRepository for SqliteRegistroIngresoRepository<'a> {
    fn crear(&self, registro: &RegistroIngreso) -> Result<i64, DatabaseError> {
        let fecha_hora_ingreso = registro
            .fecha_hora_ingreso
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();

        let medio_ingreso = match registro.medio_ingreso {
            MedioIngreso::Caminando => "CAMINANDO",
            MedioIngreso::Vehiculo => "VEHICULO",
        };

        let tipo_ingreso = match registro.tipo_ingreso {
            TipoIngreso::Praind => "PRAIND",
            TipoIngreso::InHouse => "IN_HOUSE",
            TipoIngreso::PorCorreo => "POR_CORREO",
            TipoIngreso::Swat => "SWAT",
        };

        self.connection.execute(
            "
            INSERT INTO registro_ingresos (
                contratista_id,
                empresa_id,
                fecha_hora_ingreso,
                medio_ingreso,
                tipo_ingreso,
                gafete_numero,
                usuario_ingreso_id,
                fecha_hora_salida,
                usuario_salida_id
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ",
            params![
                registro.contratista_id,
                registro.empresa_id,
                fecha_hora_ingreso,
                medio_ingreso,
                tipo_ingreso,
                registro.gafete_numero,
                registro.usuario_ingreso_id,
                registro
                    .fecha_hora_salida
                    .map(|fecha| { fecha.format("%Y-%m-%d %H:%M:%S").to_string() }),
                registro.usuario_salida_id,
            ],
        )?;

        Ok(self.connection.last_insert_rowid())
    }

    fn buscar_por_id(&self, id: i64) -> Result<Option<RegistroIngreso>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                id,
                contratista_id,
                empresa_id,
                fecha_hora_ingreso,
                medio_ingreso,
                tipo_ingreso,
                gafete_numero,
                usuario_ingreso_id,
                fecha_hora_salida,
                usuario_salida_id
            FROM registro_ingresos
            WHERE id = ?1
            ",
        )?;

        match statement.query_row(params![id], convertir_fila) {
            Ok(registro) => Ok(Some(registro)),

            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),

            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn buscar_ingreso_activo(
        &self,
        contratista_id: i64,
    ) -> Result<Option<RegistroIngreso>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                id,
                contratista_id,
                empresa_id,
                fecha_hora_ingreso,
                medio_ingreso,
                tipo_ingreso,
                gafete_numero,
                usuario_ingreso_id,
                fecha_hora_salida,
                usuario_salida_id
            FROM registro_ingresos
            WHERE contratista_id = ?1
              AND fecha_hora_salida IS NULL
            ORDER BY fecha_hora_ingreso DESC
            LIMIT 1
            ",
        )?;

        match statement.query_row(params![contratista_id], convertir_fila) {
            Ok(registro) => Ok(Some(registro)),

            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),

            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn buscar_ingreso_activo_por_gafete(
        &self,
        gafete_numero: i64,
    ) -> Result<Option<RegistroIngreso>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                id,
                contratista_id,
                empresa_id,
                fecha_hora_ingreso,
                medio_ingreso,
                tipo_ingreso,
                gafete_numero,
                usuario_ingreso_id,
                fecha_hora_salida,
                usuario_salida_id
            FROM registro_ingresos
            WHERE gafete_numero = ?1
              AND fecha_hora_salida IS NULL
            ORDER BY fecha_hora_ingreso DESC
            LIMIT 1
            ",
        )?;

        match statement.query_row(params![gafete_numero], convertir_fila) {
            Ok(registro) => Ok(Some(registro)),

            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),

            Err(error) => Err(DatabaseError::from(error)),
        }
    }

    fn registrar_salida(
        &self,
        id: i64,
        fecha_hora_salida: NaiveDateTime,
        usuario_salida_id: i64,
    ) -> Result<(), DatabaseError> {
        let fecha_hora_salida = fecha_hora_salida.format("%Y-%m-%d %H:%M:%S").to_string();

        let rows_affected = self.connection.execute(
            "
            UPDATE registro_ingresos
            SET
                fecha_hora_salida = ?1,
                usuario_salida_id = ?2
            WHERE id = ?3
              AND fecha_hora_salida IS NULL
            ",
            params![fecha_hora_salida, usuario_salida_id, id,],
        )?;

        if rows_affected == 0 {
            return Err(DatabaseError::RegistroNoActivo);
        }

        Ok(())
    }

    fn listar(&self) -> Result<Vec<RegistroIngreso>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                id,
                contratista_id,
                empresa_id,
                fecha_hora_ingreso,
                medio_ingreso,
                tipo_ingreso,
                gafete_numero,
                usuario_ingreso_id,
                fecha_hora_salida,
                usuario_salida_id
            FROM registro_ingresos
            ORDER BY fecha_hora_ingreso DESC
            ",
        )?;

        let registros = statement
            .query_map([], convertir_fila)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(registros)
    }
}
