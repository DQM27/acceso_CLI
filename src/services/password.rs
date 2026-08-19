use argon2::Argon2;
use argon2::password_hash::{
    PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng,
};

use super::error::PasswordError;

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
