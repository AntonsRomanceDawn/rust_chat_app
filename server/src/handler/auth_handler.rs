use axum::{Json, extract::State};
use chrono::{Duration, Utc};
use tracing::{info, instrument};

use crate::{
    config::AppState,
    database::{models::UserRole, refresh_token::RefreshTokenRepository, users::UserRepository},
    dtos::{
        LoginReqDto, LoginRespDto, RefreshTokenReqDto, RefreshTokenRespDto, RegisterReqDto,
        RegisterRespDto,
    },
    errors::error::AppError,
    utils::{
        hash::{hash_data, hash_password, verify_hashed_password},
        token::{generate_access_token, generate_refresh_token},
    },
};

#[instrument(skip(state, body), fields(username = %body.username))]
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterReqDto>,
) -> Result<Json<RegisterRespDto>, AppError> {
    info!("Registering new user");
    body.validate().map_err(AppError::Validation)?;

    let password_hash = hash_password(body.password)?;

    match state
        .db
        .insert_user(&body.username, &password_hash, UserRole::User)
        .await?
    {
        Some(user) => {
            let register_response = RegisterRespDto {
                id: user.id,
                username: user.username,
                role: user.role,
                created_at: user.created_at,
            };
            Ok(Json::<RegisterRespDto>(register_response))
        }
        None => Err(AppError::UsernameAlreadyExists),
    }
}

#[instrument(skip(state, body), fields(username = %body.username))]
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginReqDto>,
) -> Result<Json<LoginRespDto>, AppError> {
    info!("User logging in");
    body.validate().map_err(AppError::Validation)?;
    let user = match state.db.get_user_by_username(&body.username).await? {
        Some(user) => user,
        None => return Err(AppError::WrongCredentials),
    };

    if !verify_hashed_password(&body.password, &user.password_hash)? {
        return Err(AppError::WrongCredentials);
    }

    let access_token = generate_access_token(
        user.id,
        UserRole::User,
        state.config.jwt_secret.as_bytes(),
        state.config.access_expiry,
    )?;
    let refresh_token = generate_refresh_token()?;
    let refresh_token_hash = hash_data(&refresh_token.as_bytes());

    let _ = state
        .db
        .insert_refresh_token_by_hash(
            user.id,
            &refresh_token_hash,
            Duration::seconds(state.config.refresh_expiry),
        )
        .await?;

    let login_response = LoginRespDto {
        access_token,
        refresh_token,
    };
    Ok(Json::<LoginRespDto>(login_response))
}

#[instrument(skip(state, body))]
pub async fn refresh_token(
    State(state): State<AppState>,
    Json(body): Json<RefreshTokenReqDto>,
) -> Result<Json<RefreshTokenRespDto>, AppError> {
    info!("Refreshing token");
    let refresh_token_hash = hash_data(&body.refresh_token.as_bytes());
    let refresh_token = match state
        .db
        .get_refresh_token_by_hash(&refresh_token_hash)
        .await?
    {
        Some(token) => token,
        None => return Err(AppError::SessionExpired),
    };
    if refresh_token.expires_at < Utc::now() {
        return Err(AppError::SessionExpired);
    }

    let user_id = refresh_token.user_id;

    let _ = state
        .db
        .delete_refresh_token_by_hash(&refresh_token_hash)
        .await?;

    let access_token = generate_access_token(
        user_id,
        UserRole::User,
        state.config.jwt_secret.as_bytes(),
        state.config.access_expiry,
    )?;
    let refresh_token = generate_refresh_token()?;
    let refresh_token_hash = hash_data(&refresh_token.as_bytes());

    let _ = state
        .db
        .insert_refresh_token_by_hash(
            user_id,
            &refresh_token_hash,
            Duration::seconds(state.config.refresh_expiry),
        )
        .await?;

    let refresh_response = RefreshTokenRespDto {
        access_token,
        refresh_token,
    };
    Ok(Json::<RefreshTokenRespDto>(refresh_response))
}
