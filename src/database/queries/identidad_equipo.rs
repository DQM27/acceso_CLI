//! Unidad y etiqueta con que el panel registró este equipo (ver
//! `TokenDispositivo::sitio_nombre`), guardadas para mostrarlas en el login
//! y la barra de estado aunque no haya conexión. Sólo informativas: ningún
//! permiso depende de ellas (la unidad real la decide la nube a partir del
//! registro del equipo).

use rusqlite::{Connection, OptionalExtension};

use crate::database::error::DatabaseError;

/// Lo que se muestra del equipo. Cada parte puede faltar por separado.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IdentidadEquipo {
    pub unidad: Option<String>,
    pub etiqueta: Option<String>,
}

impl IdentidadEquipo {
    pub fn esta_vacia(&self) -> bool {
        self.unidad.is_none() && self.etiqueta.is_none()
    }
}

/// Vacía si nunca llegó del servidor.
pub fn leer(connection: &Connection) -> Result<IdentidadEquipo, DatabaseError> {
    Ok(connection
        .query_row(
            "SELECT unidad_nombre, equipo_etiqueta FROM sincronizacion_estado WHERE id = 1",
            [],
            |row| {
                Ok(IdentidadEquipo {
                    unidad: row.get(0)?,
                    etiqueta: row.get(1)?,
                })
            },
        )
        .optional()?
        .unwrap_or_default())
}

pub fn guardar(connection: &Connection, identidad: &IdentidadEquipo) -> Result<(), DatabaseError> {
    connection.execute(
        "UPDATE sincronizacion_estado SET unidad_nombre = ?1, equipo_etiqueta = ?2 WHERE id = 1",
        (&identidad.unidad, &identidad.etiqueta),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::initialize_database;

    #[test]
    fn sin_datos_esta_vacia_y_guardar_la_reemplaza() {
        let connection = Connection::open_in_memory().unwrap();
        initialize_database(&connection).unwrap();
        assert!(leer(&connection).unwrap().esta_vacia());

        let identidad = IdentidadEquipo {
            unidad: Some("Planta Cartago".to_string()),
            etiqueta: Some("PC portería norte".to_string()),
        };
        guardar(&connection, &identidad).unwrap();
        assert_eq!(leer(&connection).unwrap(), identidad);
    }
}
