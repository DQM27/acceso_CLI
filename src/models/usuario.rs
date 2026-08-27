#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Usuario {
    pub id: i64,
    pub cedula: String,
    pub nombre: String,
    pub password_hash: String,
    pub rol: RolUsuario,
    pub activo: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RolUsuario {
    Root,
    Administrador,
    Operador,
}
