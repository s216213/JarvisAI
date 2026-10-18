use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};

use crate::error::{AppError, AppResult};

pub fn hash_password(password: &str) -> AppResult<String> {
    let hash = Argon2::default()
        .hash_password(password.as_bytes())
        .map_err(|e| AppError::Anyhow(anyhow::anyhow!(e)))?
        .to_string();
    Ok(hash)
}

pub fn verify_password(password: &str, password_hash: &str) -> AppResult<bool> {
    let parsed =
        PasswordHash::new(password_hash).map_err(|e| AppError::Anyhow(anyhow::anyhow!(e)))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}
