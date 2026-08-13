use std::path::{Path, PathBuf};

use rusqlite::Connection;

use super::schema::initialize_database;

pub const DATABASE_PATH_ENV: &str = "CONTROL_ACCESO_DB";

pub fn ruta_base_datos() -> PathBuf {
    std::env::var_os(DATABASE_PATH_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("control_acceso.db"))
}

/// Abre la base productiva y aplica toda su inicialización en una única ruta.
pub fn open_database(path: impl AsRef<Path>) -> rusqlite::Result<Connection> {
    let connection = Connection::open(path)?;
    initialize_database(&connection)?;
    Ok(connection)
}
