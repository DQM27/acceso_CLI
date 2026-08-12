use crate::database::repositories::usuario_repository::UsuarioRepository;
use crate::models::usuario::Usuario;

use super::error::AutenticacionError;
use super::password::verificar_password;

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

    pub fn autenticar(&self, cedula: &str, password: &str) -> Result<Usuario, AutenticacionError> {
        let usuario = self
            .usuarios
            .buscar_por_cedula(cedula.trim())?
            .ok_or(AutenticacionError::CredencialesInvalidas)?;

        if !usuario.activo {
            return Err(AutenticacionError::UsuarioInactivo);
        }

        match verificar_password(password, &usuario.password_hash) {
            Ok(true) => Ok(usuario),
            Ok(false) => Err(AutenticacionError::CredencialesInvalidas),
            Err(_) => Err(AutenticacionError::HashInvalido),
        }
    }
}
