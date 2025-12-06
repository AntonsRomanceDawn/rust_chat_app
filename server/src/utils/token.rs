use base64::Engine;
use chrono::{Duration, Utc};
use scrypt::password_hash::rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use tracing::{error, instrument, warn};
use uuid::Uuid;

use crate::{
    database::models::UserRole,
    errors::{error::ApiErrorItem, error::HttpError, error_codes},
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub role: UserRole,
    pub iat: usize,
    pub exp: usize,
}

#[instrument(skip(secret))]
pub fn generate_access_token(
    user_id: Uuid,
    role: UserRole,
    secret: &[u8],
    expires_in_seconds: i64,
) -> Result<String, HttpError> {
    let now = Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + Duration::seconds(expires_in_seconds)).timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        role,
        iat,
        exp,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )
    .map_err(|e| {
        error!("Failed to generate access token: {:?}", e);
        HttpError::internal([ApiErrorItem::new(error_codes::INTERNAL_SERVER_ERROR, None)])
    })
}

#[instrument]
pub fn generate_refresh_token() -> Result<String, HttpError> {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes))
}

pub fn verify_access_token(token: &str, secret: &[u8]) -> Result<(Uuid, usize), HttpError> {
    let result = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret),
        &Validation::default(),
    );

    match result {
        Ok(token_data) => {
            let user_id = Uuid::parse_str(&token_data.claims.sub).map_err(|e| {
                error!("Failed to parse user ID from token claims: {:?}", e);
                HttpError::internal([ApiErrorItem::new(error_codes::INTERNAL_SERVER_ERROR, None)])
            })?;
            Ok((user_id, token_data.claims.exp))
        }

        Err(e) => {
            warn!("Failed to verify access token: {:?}", e);
            Err(HttpError::unauthorized([ApiErrorItem::new(
                error_codes::INVALID_TOKEN,
                None,
            )]))
        }
    }
}
