//! Último desfase medido entre el reloj del equipo y la hora del servidor
//! (ver `tiempo::RelojCorregido`), guardado para que sobreviva al cierre de
//! la app: sin esto, cada arranque volvía a sellar movimientos con el reloj
//! del equipo hasta la primera respuesta de la nube, y un arranque sin
//! internet lo hacía durante todo el turno.

use rusqlite::{Connection, OptionalExtension};

use crate::database::error::DatabaseError;
use crate::tiempo::Ancla;

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

/// Ancla del reloj confiable guardada (ver `tiempo::Ancla`), o `None`.
pub fn leer_ancla(connection: &Connection) -> Result<Option<Ancla>, DatabaseError> {
    Ok(connection
        .query_row(
            "SELECT ancla_servidor_ms, ancla_arranque_ms, ancla_epoca_arranque_ms
               FROM sincronizacion_estado WHERE id = 1",
            [],
            |row| {
                Ok((
                    row.get::<_, Option<i64>>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            },
        )
        .optional()?
        .and_then(|fila| match fila {
            (Some(servidor_ms), Some(arranque_ms), Some(epoca_arranque_ms)) => Some(Ancla {
                servidor_ms,
                arranque_ms: u64::try_from(arranque_ms).ok()?,
                epoca_arranque_ms,
            }),
            _ => None,
        }))
}

pub fn guardar_ancla(connection: &Connection, ancla: &Ancla) -> Result<(), DatabaseError> {
    // `arranque_ms` cabe en i64 por siglos (ms desde el arranque); si no
    // cupiera, se guarda sin ancla en lugar de un valor truncado.
    let arranque_ms = i64::try_from(ancla.arranque_ms).ok();
    connection.execute(
        "UPDATE sincronizacion_estado
            SET ancla_servidor_ms = ?1, ancla_arranque_ms = ?2, ancla_epoca_arranque_ms = ?3
          WHERE id = 1",
        (ancla.servidor_ms, arranque_ms, ancla.epoca_arranque_ms),
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

    #[test]
    fn el_ancla_se_guarda_y_se_lee_entera() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        assert_eq!(leer_ancla(&connection).unwrap(), None);

        let ancla = Ancla {
            servidor_ms: 1_790_000_000_000,
            arranque_ms: 3_600_000,
            epoca_arranque_ms: 1_789_996_400_000,
        };
        guardar_ancla(&connection, &ancla).unwrap();
        assert_eq!(leer_ancla(&connection).unwrap(), Some(ancla));
    }
}
