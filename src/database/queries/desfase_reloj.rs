//! Último desfase medido entre el reloj del equipo y la hora del servidor
//! (ver `tiempo::RelojCorregido`), guardado para que sobreviva al cierre de
//! la app: sin esto, cada arranque volvía a sellar movimientos con el reloj
//! del equipo hasta la primera respuesta de la nube, y un arranque sin
//! internet lo hacía durante todo el turno.

use rusqlite::{Connection, OptionalExtension};

use crate::database::error::DatabaseError;

/// `None` si nunca se midió.
pub fn leer(connection: &Connection) -> Result<Option<i64>, DatabaseError> {
    Ok(connection
        .query_row(
            "SELECT desfase_reloj_ms FROM sincronizacion_estado WHERE id = 1",
            [],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()?
        .flatten())
}

pub fn guardar(connection: &Connection, desfase_ms: i64) -> Result<(), DatabaseError> {
    connection.execute(
        "UPDATE sincronizacion_estado SET desfase_reloj_ms = ?1 WHERE id = 1",
        [desfase_ms],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::initialize_database;

    #[test]
    fn sin_medicion_no_hay_desfase_y_guardar_lo_sobrescribe() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();

        assert_eq!(leer(&connection).unwrap(), None);

        guardar(&connection, 60_000).unwrap();
        assert_eq!(leer(&connection).unwrap(), Some(60_000));

        guardar(&connection, -1_500).unwrap();
        assert_eq!(leer(&connection).unwrap(), Some(-1_500));
    }
}
