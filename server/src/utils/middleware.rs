use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use tracing::warn;
use uuid::Uuid;

use crate::{
    config::AppState, database::models::UserRole, errors::error::AppError,
    utils::token::verify_access_token,
};

/// This struct represents a successfully authenticated user.
/// Adding this as an argument to a handler will force authentication.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub role: UserRole,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // 1. Extract the Authorization header
        let auth_header = parts.headers.get(header::AUTHORIZATION).ok_or_else(|| {
            warn!("Missing authorization header");
            AppError::InvalidRequestFormat
        })?;

        let auth_str = auth_header.to_str().map_err(|_| {
            warn!("Invalid authorization header encoding");
            AppError::InvalidRequestFormat
        })?;

        // 2. Verify we have "Bearer <token>"
        if !auth_str.starts_with("Bearer ") {
            warn!("Invalid authorization header format (missing Bearer)");
            return Err(AppError::InvalidRequestFormat);
        }

        let token = &auth_str[7..];

        // 3. Verify the JWT
        let (user_id, role, _) = verify_access_token(token, state.config.jwt_secret.as_bytes())?;

        Ok(AuthUser { user_id, role })
    }
}
