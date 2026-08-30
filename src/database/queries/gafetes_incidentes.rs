//! Historial append-only de incidentes de gafetes (`docs/plan-gafetes.md`),
//! mismo patrón que `queries::auditoria`: `gafetes` guarda sólo el estado
//! vigente, acá queda el rastro de cuándo se marcó perdido y cuándo se
//! resolvió, y con qué motivo.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};

use crate::database::error::DatabaseError;
use crate::models::gafete::MotivoResolucionGafete;
use crate::tiempo::serializar_utc;

pub trait GafetesIncidentesWriter {
    fn registrar_perdido(
        &self,
        gafete_id: i64,
        fecha_hora: DateTime<Utc>,
        usuario_id: i64,
        contratista_id: i64,
    ) -> Result<(), DatabaseError>;

    fn registrar_resuelto(
        &self,
        gafete_id: i64,
        fecha_hora: DateTime<Utc>,
        usuario_id: i64,
        motivo: MotivoResolucionGafete,
    ) -> Result<(), DatabaseError>;
}

pub struct SqliteGafetesIncidentes<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteGafetesIncidentes<'a> {
    pub fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

impl GafetesIncidentesWriter for SqliteGafetesIncidentes<'_> {
    fn registrar_perdido(
        &self,
        gafete_id: i64,
        fecha_hora: DateTime<Utc>,
        usuario_id: i64,
        contratista_id: i64,
    ) -> Result<(), DatabaseError> {
        self.connection.execute(
            "INSERT INTO gafetes_incidentes (gafete_id, tipo, fecha_hora, usuario_id, contratista_id)
             VALUES (?1, 'PERDIDO', ?2, ?3, ?4)",
            params![gafete_id, serializar_utc(fecha_hora), usuario_id, contratista_id],
        )?;
        Ok(())
    }

    fn registrar_resuelto(
        &self,
        gafete_id: i64,
        fecha_hora: DateTime<Utc>,
        usuario_id: i64,
        motivo: MotivoResolucionGafete,
    ) -> Result<(), DatabaseError> {
        self.connection.execute(
            "INSERT INTO gafetes_incidentes (gafete_id, tipo, fecha_hora, usuario_id, motivo_resolucion)
             VALUES (?1, 'RESUELTO', ?2, ?3, ?4)",
            params![
                gafete_id,
                serializar_utc(fecha_hora),
                usuario_id,
                motivo.as_str_sql()
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::initialize_database;
    use rusqlite::Connection;

    fn conexion_con_gafete_y_partes() -> (Connection, i64) {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
            .execute_batch(
                "INSERT INTO empresas (nombre) VALUES ('Acme');
                 INSERT INTO contratistas (cedula, nombre, empresa_id, tipo_ingreso, es_personal_ruta, tiene_acceso)
                 VALUES ('1', 'Juan', 1, 'PRAIND', 0, 1);
                 INSERT INTO usuarios (cedula, nombre, password_hash, rol, activo)
                 VALUES ('9', 'Root', 'hash', 'ROOT', 1);
                 INSERT INTO gafetes (numero, estado) VALUES (1, 'DISPONIBLE');",
            )
            .unwrap();
        let gafete_id: i64 = connection
            .query_row("SELECT id FROM gafetes WHERE numero = 1", [], |row| {
                row.get(0)
            })
            .unwrap();
        (connection, gafete_id)
    }

    #[test]
    fn registra_perdido_y_resuelto_como_filas_distintas() {
        let (connection, gafete_id) = conexion_con_gafete_y_partes();
        let writer = SqliteGafetesIncidentes::new(&connection);
        let ahora = Utc::now();

        writer.registrar_perdido(gafete_id, ahora, 1, 1).unwrap();
        writer
            .registrar_resuelto(gafete_id, ahora, 1, MotivoResolucionGafete::Aparecido)
            .unwrap();

        let total: i64 = connection
            .query_row("SELECT COUNT(*) FROM gafetes_incidentes", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(total, 2);
    }
}
