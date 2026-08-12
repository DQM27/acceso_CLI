use crate::models::usuario::{RolUsuario, Usuario};

/// Construye una identidad únicamente en memoria para la futura CLI de desarrollo.
///
/// Su ID no existe en SQLite y por ello no puede utilizarse como `usuario_ingreso_id`
/// ni `usuario_salida_id`: las claves foráneas rechazarían esos movimientos.
pub fn usuario_desarrollo() -> Usuario {
    Usuario {
        id: 0,
        cedula: "DEV".to_string(),
        nombre: "Usuario Desarrollo".to_string(),
        password_hash: String::new(),
        rol: RolUsuario::Root,
        activo: true,
    }
}
