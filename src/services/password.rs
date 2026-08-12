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

pub fn verificar_password(password: &str, hash: &str) -> Result<bool, PasswordError> {
    let hash = PasswordHash::new(hash).map_err(|_| PasswordError::HashInvalido)?;

    match Argon2::default().verify_password(password.as_bytes(), &hash) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(_) => Err(PasswordError::HashInvalido),
    }
}
