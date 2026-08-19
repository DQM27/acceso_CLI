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

/// Resultado de resolver la cédula (existe, está activo) sin haber verificado todavía la
/// contraseña — permite separar la parte que toca SQLite (rápida) de la verificación de
/// Argon2 (lenta), para que esta última pueda correr fuera del hilo de eventos.
#[derive(Clone, PartialEq, Eq)]
pub struct CandidatoAutenticacion {
    pub sesion: UsuarioSesion,
    pub password_hash: String,
}

impl std::fmt::Debug for CandidatoAutenticacion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CandidatoAutenticacion")
            .field("sesion", &self.sesion)
            .field("password_hash", &"«redactado»")
            .finish()
    }
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
        let candidato = self.buscar_candidato(cedula)?;
        verificar_candidato(candidato, password)
    }

    /// Resuelve la cédula y confirma que el usuario está activo, sin verificar todavía la
    /// contraseña. El llamador decide dónde y cuándo correr `verificar_password` sobre el
    /// hash devuelto (por ejemplo, en un hilo aparte).
    pub fn buscar_candidato(
        &self,
        cedula: &str,
    ) -> Result<CandidatoAutenticacion, AutenticacionError> {
        let usuario = self
            .usuarios
            .buscar_por_cedula(cedula.trim())?
            .ok_or(AutenticacionError::CredencialesInvalidas)?;

        if !usuario.activo {
            return Err(AutenticacionError::UsuarioInactivo);
        }

        Ok(CandidatoAutenticacion {
            sesion: UsuarioSesion {
                id: usuario.id,
                cedula: usuario.cedula,
                nombre: usuario.nombre,
                rol: usuario.rol,
            },
            password_hash: usuario.password_hash,
        })
    }
}

/// Verifica `password` contra un candidato ya resuelto por `buscar_candidato`.
/// Función libre (no depende de `&self`/repositorio) a propósito: la usa tanto
/// `autenticar` como el hilo aparte que arma la TUI para correr Argon2 sin
/// bloquear la UI — antes ese hilo copiaba a mano el mismo `match` de aquí.
pub fn verificar_candidato(
    candidato: CandidatoAutenticacion,
    password: &str,
) -> Result<UsuarioSesion, AutenticacionError> {
    match verificar_password(password, &candidato.password_hash) {
        Ok(true) => Ok(candidato.sesion),
        Ok(false) => Err(AutenticacionError::CredencialesInvalidas),
        Err(_) => Err(AutenticacionError::HashInvalido),
    }
}
