use crate::models::usuario::RolUsuario;

use super::autenticacion_service::UsuarioSesion;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DevAuthError {
    #[error("La operación auditada requiere seleccionar o autenticar un usuario persistido")]
    ActorPersistidoRequerido,
}

/// Construye una identidad únicamente en memoria para la futura CLI de desarrollo.
///
/// Su ID no existe en `SQLite` y por ello no puede utilizarse como `usuario_ingreso_id`
/// ni `usuario_salida_id`: las claves foráneas rechazarían esos movimientos.
pub fn usuario_desarrollo() -> UsuarioSesion {
    UsuarioSesion {
        id: 0,
        cedula: "DEV".to_string(),
        nombre: "Usuario Desarrollo".to_string(),
        rol: RolUsuario::Root,
    }
}

/// Obtiene el ID que puede usarse como actor de una operación auditada.
///
/// La identidad de navegación de desarrollo se rechaza explícitamente. La futura CLI
/// deberá seleccionar o autenticar un usuario persistido antes de registrar movimientos.
pub fn actor_persistido(usuario: &UsuarioSesion) -> Result<i64, DevAuthError> {
    if usuario.id <= 0 {
        return Err(DevAuthError::ActorPersistidoRequerido);
    }

    Ok(usuario.id)
}
