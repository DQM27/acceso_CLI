//! Estado de este equipo ante la nube, en `sincronizacion_estado` (ver
//! `MIGRACION_59` y docs/features-futuras/plan-reconexion-equipo-retirado.md):
//! su unidad, si lo retiraron en el panel y si su base es de una unidad
//! anterior y hay que descartarla.

use rusqlite::{Connection, OptionalExtension};

use crate::database::error::DatabaseError;

fn leer_bandera(connection: &Connection, columna: &str) -> Result<bool, DatabaseError> {
    Ok(connection
        .query_row(
            &format!("SELECT {columna} FROM sincronizacion_estado WHERE id = 1"),
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .is_some_and(|valor| valor == 1))
}

fn guardar_bandera(
    connection: &Connection,
    columna: &str,
    valor: bool,
) -> Result<(), DatabaseError> {
    connection.execute(
        &format!("UPDATE sincronizacion_estado SET {columna} = ?1 WHERE id = 1"),
        [i64::from(valor)],
    )?;
    Ok(())
}

/// Unidad (`sitio_id`) del último token válido; `None` si todavía no llegó
/// ninguno a esta base.
pub fn sitio_id(connection: &Connection) -> Result<Option<String>, DatabaseError> {
    Ok(connection
        .query_row(
            "SELECT sitio_id FROM sincronizacion_estado WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .optional()?
        .flatten())
}

pub fn guardar_sitio_id(connection: &Connection, sitio_id: &str) -> Result<(), DatabaseError> {
    connection.execute(
        "UPDATE sincronizacion_estado SET sitio_id = ?1 WHERE id = 1",
        [sitio_id],
    )?;
    Ok(())
}

/// `true` si la nube dijo que no reconoce la clave de este equipo.
pub fn equipo_retirado(connection: &Connection) -> Result<bool, DatabaseError> {
    leer_bandera(connection, "equipo_retirado")
}

pub fn marcar_retirado(connection: &Connection, retirado: bool) -> Result<(), DatabaseError> {
    guardar_bandera(connection, "equipo_retirado", retirado)
}

/// `true` si esta base es de una unidad anterior: no se sincroniza y se
/// borra al arrancar.
pub fn base_por_descartar(connection: &Connection) -> Result<bool, DatabaseError> {
    leer_bandera(connection, "descartar_base")
}

pub fn marcar_base_por_descartar(
    connection: &Connection,
    descartar: bool,
) -> Result<(), DatabaseError> {
    guardar_bandera(connection, "descartar_base", descartar)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::initialize_database;

    fn base() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        connection
    }

    #[test]
    fn una_base_nueva_no_tiene_unidad_ni_marcas() {
        let connection = base();
        assert_eq!(sitio_id(&connection).unwrap(), None);
        assert!(!equipo_retirado(&connection).unwrap());
        assert!(!base_por_descartar(&connection).unwrap());
    }

    #[test]
    fn guarda_y_borra_cada_marca_por_separado() {
        let connection = base();
        guardar_sitio_id(&connection, "s1").unwrap();
        marcar_retirado(&connection, true).unwrap();
        assert_eq!(sitio_id(&connection).unwrap().as_deref(), Some("s1"));
        assert!(equipo_retirado(&connection).unwrap());
        assert!(!base_por_descartar(&connection).unwrap());

        marcar_base_por_descartar(&connection, true).unwrap();
        marcar_retirado(&connection, false).unwrap();
        assert!(!equipo_retirado(&connection).unwrap());
        assert!(base_por_descartar(&connection).unwrap());
    }
}
