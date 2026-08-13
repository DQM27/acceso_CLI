use crate::database::repositories::usuario_repository::UsuarioRepository;
use crate::models::usuario::RolUsuario;

use super::error::AutenticacionError;
use super::password::verificar_password;

/// Identidad autenticada que puede cruzar hacia aplicación/presentación sin exponer el hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsuarioSesion {
    pub id: i64,
    pub cedula: String,
    pub nombre: String,
    pub rol: RolUsuario,
}

pub struct AutenticacionService<'a, R>
where
    R: UsuarioRepository + ?Sized,
{
    usuarios: &'a R,
}

impl<'a, R> AutenticacionService<'a, R>
where
    R: UsuarioRepository + ?Sized,
{
    pub fn new(usuarios: &'a R) -> Self {
        Self { usuarios }
    }

    pub fn autenticar(
        &self,
        cedula: &str,
        password: &str,
    ) -> Result<UsuarioSesion, AutenticacionError> {
        let usuario = self
            .usuarios
            .buscar_por_cedula(cedula.trim())?
            .ok_or(AutenticacionError::CredencialesInvalidas)?;

        if !usuario.activo {
            return Err(AutenticacionError::UsuarioInactivo);
        }

        match verificar_password(password, &usuario.password_hash) {
            Ok(true) => Ok(UsuarioSesion {
                id: usuario.id,
                cedula: usuario.cedula,
                nombre: usuario.nombre,
                rol: usuario.rol,
            }),
            Ok(false) => Err(AutenticacionError::CredencialesInvalidas),
            Err(_) => Err(AutenticacionError::HashInvalido),
        }
    }
}
