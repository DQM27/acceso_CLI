//! Encolar hacia la bandeja de salida (`docs/plan-persistencia-nube.md`).
//! Un solo punto de entrada para que los repositorios que crean/cierran
//! contratistas e ingresos no repitan el mismo `INSERT`.

use rusqlite::{Connection, params};

/// Agrega una fila `pendiente` a `cola_salida`. Se llama siempre dentro de
/// la misma transacción que crea/actualiza la fila real -- si la
/// transacción se revierte, la fila de cola se revierte con ella, así que
/// nunca puede quedar un cambio real sin su aviso correspondiente.
pub fn encolar(
    connection: &Connection,
    entidad: &str,
    entidad_uuid: &str,
    operacion: &str,
) -> rusqlite::Result<()> {
    connection.execute(
        "
        INSERT INTO cola_salida (
            entidad, entidad_uuid, operacion, creado_en, actualizado_en
        )
        VALUES (
            ?1, ?2, ?3,
            strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
            strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
        )
        ",
        params![entidad, entidad_uuid, operacion],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::*;
    use crate::database::schema::initialize_database;

    fn conexion_de_prueba() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
    }

    #[test]
    fn encola_una_fila_pendiente() {
        let connection = conexion_de_prueba();

        encolar(&connection, "contratista", "un-uuid", "crear").unwrap();

        let (entidad, entidad_uuid, operacion, estado, intentos): (
            String,
            String,
            String,
            String,
            i64,
        ) = connection
            .query_row(
                "SELECT entidad, entidad_uuid, operacion, estado, intentos FROM cola_salida",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .unwrap();

        assert_eq!(entidad, "contratista");
        assert_eq!(entidad_uuid, "un-uuid");
        assert_eq!(operacion, "crear");
        assert_eq!(estado, "pendiente");
        assert_eq!(intentos, 0);
    }
}
