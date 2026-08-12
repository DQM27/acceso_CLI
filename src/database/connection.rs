use rusqlite::Connection;

pub fn open_database(path: &str) -> rusqlite::Result<Connection> {
    let connection = Connection::open(path)?;

    connection.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        ",
    )?;

    Ok(connection)
}