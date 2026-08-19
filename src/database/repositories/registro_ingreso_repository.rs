use chrono::{DateTime, Utc};
use rusqlite::{Connection, Row, named_params, params};

use crate::database::error::DatabaseError;
use crate::models::medio_ingreso::MedioIngreso;
use crate::models::registro_ingreso::{
    MotivoResultadoIngreso, NuevoRegistroIngreso, RegistroIngreso, ResultadoIngresoRegistrado,
};
use crate::models::tipo_ingreso::TipoIngreso;
use crate::tiempo::{parsear_utc, serializar_utc};

pub trait RegistroIngresoRepository {
    fn crear(&self, registro: &NuevoRegistroIngreso) -> Result<i64, DatabaseError>;

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
        fecha_hora_salida: DateTime<Utc>,
        usuario_salida_id: i64,
    ) -> Result<(), DatabaseError>;

    fn listar(&self) -> Result<Vec<RegistroIngreso>, DatabaseError>;
}

fn motivo_a_texto(motivo: MotivoResultadoIngreso) -> &'static str {
    match motivo {
        MotivoResultadoIngreso::PraindProximoVencer => "PRAIND_PROXIMO_VENCER",
        MotivoResultadoIngreso::DatosReconstruidos => "DATOS_RECONSTRUIDOS",
    }
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

    let fecha_hora_ingreso = parsear_utc(&fecha_hora_ingreso_texto).map_err(|error| {
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
            parsear_utc(&fecha).map_err(|error| {
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
    fn crear(&self, registro: &NuevoRegistroIngreso) -> Result<i64, DatabaseError> {
        let fecha_hora_ingreso = serializar_utc(registro.fecha_hora_ingreso);

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

        let fecha_vencimiento_praind = registro
            .datos_historicos
            .fecha_vencimiento_praind
            .map(|fecha| fecha.format("%Y-%m-%d").to_string());
        let (resultado_acceso, motivo_resultado) = match registro.datos_historicos.resultado_acceso
        {
            ResultadoIngresoRegistrado::Permitido => ("PERMITIDO", None),
            ResultadoIngresoRegistrado::PermitidoConAdvertencia(motivo) => {
                ("PERMITIDO_CON_ADVERTENCIA", Some(motivo_a_texto(motivo)))
            }
            ResultadoIngresoRegistrado::Migrado => ("MIGRADO", Some("DATOS_RECONSTRUIDOS")),
        };

        let filas = self.connection.execute(
            "
            INSERT INTO registro_ingresos (
                contratista_id,
                empresa_id,
                fecha_hora_ingreso,
                medio_ingreso,
                tipo_ingreso,
                gafete_numero,
                usuario_ingreso_id,
                contratista_cedula,
                contratista_nombre,
                empresa_nombre,
                usuario_ingreso_nombre,
                fecha_vencimiento_praind,
                es_personal_ruta,
                tiene_acceso,
                resultado_acceso,
                motivo_resultado,
                reglas_version
            )
            SELECT
                :contratista_id, :empresa_id, :fecha_hora_ingreso,
                :medio_ingreso, :tipo_ingreso, :gafete_numero,
                :usuario_ingreso_id, :contratista_cedula,
                :contratista_nombre, e.nombre, u.nombre,
                :fecha_vencimiento_praind, :es_personal_ruta,
                :tiene_acceso, :resultado_acceso, :motivo_resultado,
                :reglas_version
            FROM empresas AS e
            CROSS JOIN usuarios AS u
            WHERE e.id = :empresa_id AND u.id = :usuario_ingreso_id
            ",
            named_params! {
                ":contratista_id": registro.contratista_id,
                ":empresa_id": registro.empresa_id,
                ":fecha_hora_ingreso": fecha_hora_ingreso,
                ":medio_ingreso": medio_ingreso,
                ":tipo_ingreso": tipo_ingreso,
                ":gafete_numero": registro.gafete_numero,
                ":usuario_ingreso_id": registro.usuario_ingreso_id,
                ":contratista_cedula": registro.datos_historicos.contratista_cedula,
                ":contratista_nombre": registro.datos_historicos.contratista_nombre,
                ":fecha_vencimiento_praind": fecha_vencimiento_praind,
                ":es_personal_ruta": registro.datos_historicos.es_personal_ruta as i64,
                ":tiene_acceso": registro.datos_historicos.tiene_acceso as i64,
                ":resultado_acceso": resultado_acceso,
                ":motivo_resultado": motivo_resultado,
                ":reglas_version": registro.datos_historicos.reglas_version,
            },
        )?;

        if filas == 0 {
            return Err(DatabaseError::Sqlite(rusqlite::Error::QueryReturnedNoRows));
        }

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
        fecha_hora_salida: DateTime<Utc>,
        usuario_salida_id: i64,
    ) -> Result<(), DatabaseError> {
        let fecha_hora_salida = serializar_utc(fecha_hora_salida);

        let rows_affected = self.connection.execute(
            "
            UPDATE registro_ingresos
            SET
                fecha_hora_salida = ?1,
                usuario_salida_id = ?2,
                usuario_salida_nombre = (SELECT nombre FROM usuarios WHERE id = ?2)
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
