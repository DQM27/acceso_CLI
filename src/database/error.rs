#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Error de SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("No existe un registro de ingreso activo con ese ID")]
    RegistroNoActivo,
    #[error("La configuración inicial ya fue realizada")]
    ConfiguracionInicialYaRealizada,
    #[error("Usuario no encontrado")]
    UsuarioNoEncontrado,
    #[error("No se puede desactivar o degradar al último ROOT activo")]
    UltimoRootActivo,
    /// Una fecha almacenada no se pudo parsear. No es un error de `SQLite` —
    /// `SQLite` ya devolvió el texto sin problema; el fallo es al interpretarlo
    /// como fecha/hora.
    #[error("Fecha almacenada inválida: {0}")]
    FechaCorrupta(String),
}

impl DatabaseError {
    /// Detecta una violación de restricción `UNIQUE` (cédula/nombre duplicado,
    /// etc.), sin importar cuál columna — el llamador ya sabe cuál era.
    /// Antes copiado igual en 3 servicios (usuario, contratista, empresa).
    pub fn es_constraint_unique(&self) -> bool {
        matches!(
            self,
            Self::Sqlite(rusqlite::Error::SqliteFailure(codigo, _))
                if codigo.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
        )
    }
}
