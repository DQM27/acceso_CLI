use argon2::Argon2;
use argon2::password_hash::{
    PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng,
};

use super::error::PasswordError;

/// Hash "vacío" para un usuario global (llegó por `recibir_usuarios`, ver
/// `nube::sincronizacion`) que todavía no fijó contraseña EN ESTE
/// dispositivo -- no es un hash real, nunca puede pasar `PasswordHash::new`
/// (cualquier string sin el formato PHC cae ahí), que es justo la señal
/// que `AutenticacionService::buscar_candidato` usa para mandar al alta de
/// contraseña en vez de tratarlo como credenciales inválidas. Constante
/// explícita (no `""`) para que grepear el string alcance para encontrar
/// todo lo que lo usa.
pub const SIN_PASSWORD_LOCAL: &str = "SIN_PASSWORD_LOCAL";

pub fn generar_hash(password: &str) -> Result<String, PasswordError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| PasswordError::GeneracionHash)
}

/// Rechaza cualquier cosa que no tenga el formato PHC de un hash real (p. ej. un
/// password en texto plano pasado por error a un `..._con_hash`) sin volver a
/// calcularlo — sólo valida la forma del string, no lo verifica contra ningún
/// password.
pub fn validar_formato_hash(hash: &str) -> Result<(), PasswordError> {
    PasswordHash::new(hash)
        .map(|_| ())
        .map_err(|_| PasswordError::HashInvalido)
}

pub fn verificar_password(password: &str, hash: &str) -> Result<bool, PasswordError> {
    let hash = PasswordHash::new(hash).map_err(|_| PasswordError::HashInvalido)?;

    match Argon2::default().verify_password(password.as_bytes(), &hash) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(_) => Err(PasswordError::HashInvalido),
    }
}
