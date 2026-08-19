#[derive(Debug)]
pub enum DatabaseError {
    Sqlite(rusqlite::Error),
    RegistroNoActivo,
    ConfiguracionInicialYaRealizada,
    UsuarioNoEncontrado,
    UltimoRootActivo,
    /// Una fecha almacenada no se pudo parsear. No es un error de SQLite —
    /// SQLite ya devolvió el texto sin problema; el fallo es al interpretarlo
    /// como fecha/hora.
    FechaCorrupta(String),
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
            DatabaseError::ConfiguracionInicialYaRealizada => {
                write!(formatter, "La configuración inicial ya fue realizada")
            }
            DatabaseError::UsuarioNoEncontrado => write!(formatter, "Usuario no encontrado"),
            DatabaseError::UltimoRootActivo => write!(
                formatter,
                "No se puede desactivar o degradar al último ROOT activo"
            ),
            DatabaseError::FechaCorrupta(detalle) => {
                write!(formatter, "Fecha almacenada inválida: {detalle}")
            }
        }
    }
}

impl DatabaseError {
    /// Detecta una violación de restricción `UNIQUE` (cédula/nombre duplicado,
    /// etc.), sin importar cuál columna — el llamador ya sabe cuál era.
    /// Antes copiado igual en 3 servicios (usuario, contratista, empresa).
    pub fn es_constraint_unique(&self) -> bool {
        matches!(
            self,
            DatabaseError::Sqlite(rusqlite::Error::SqliteFailure(codigo, _))
                if codigo.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
        )
    }
}

impl std::error::Error for DatabaseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DatabaseError::Sqlite(error) => Some(error),
            DatabaseError::RegistroNoActivo
            | DatabaseError::ConfiguracionInicialYaRealizada
            | DatabaseError::UsuarioNoEncontrado
            | DatabaseError::UltimoRootActivo
            | DatabaseError::FechaCorrupta(_) => None,
        }
    }
}

impl From<rusqlite::Error> for DatabaseError {
    fn from(error: rusqlite::Error) -> Self {
        DatabaseError::Sqlite(error)
    }
}
