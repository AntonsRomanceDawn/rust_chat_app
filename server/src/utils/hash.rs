use scrypt::{
    Scrypt,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use sha2::{Digest, Sha256};
use tracing::{error, instrument};

use crate::errors::{
    error::{ApiErrorItem, HttpError},
    error_codes,
};

#[instrument(skip(password))]
pub fn hash_password(password: impl Into<String>) -> Result<String, HttpError> {
    let password = password.into();

    let salt = SaltString::generate(&mut OsRng);

    let hashed_password = Scrypt
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| {
            error!("Failed to hash password: {:?}", e);
            HttpError::internal([ApiErrorItem::new(error_codes::INTERNAL_SERVER_ERROR, None)])
        })?;

    Ok(hashed_password.to_string())
}

#[instrument(skip(password, password_hash))]
pub fn verify_hashed_password(password: &str, password_hash: &str) -> Result<bool, HttpError> {
    let stored_hash = PasswordHash::new(password_hash).map_err(|e| {
        error!("Failed to parse stored password hash: {:?}", e);
        HttpError::internal([ApiErrorItem::new(error_codes::INTERNAL_SERVER_ERROR, None)])
    })?;

    let is_valid = Scrypt
        .verify_password(password.as_bytes(), &stored_hash)
        .is_ok();

    Ok(is_valid)
}

#[instrument(skip(token))]
pub fn hash_refresh_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}
