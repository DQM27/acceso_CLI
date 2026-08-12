#[derive(Debug)]
pub enum DatabaseError {
    Sqlite(rusqlite::Error),
    RegistroNoActivo,
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseError::Sqlite(error) => {
                write!(formatter, "Error de SQLite: {}", error)
            }
            DatabaseError::RegistroNoActivo => {
                write!(
                    formatter,
                    "No existe un registro de ingreso activo con ese ID"
                )
            }
        }
    }
}

impl std::error::Error for DatabaseError {}

impl From<rusqlite::Error> for DatabaseError {
    fn from(error: rusqlite::Error) -> Self {
        DatabaseError::Sqlite(error)
    }
}
