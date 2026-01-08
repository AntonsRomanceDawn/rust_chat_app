use axum::{
    Json,
    extract::{Path, State},
};
use tracing::{info, instrument, warn};

use crate::{
    config::AppState,
    database::{keys::KeyRepository, users::UserRepository},
    dtos::{
        KeyCountRespDto, OneTimePreKeyDto, PreKeyBundleRespDto, SignedPreKeyDto, UploadKeysReqDto,
    },
    errors::error::AppError,
    utils::middleware::AuthUser,
};

#[instrument(skip(state, body))]
pub async fn upload_keys(
    user: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<UploadKeysReqDto>,
) -> Result<(), AppError> {
    info!("Uploading keys for user {}", user.user_id);

    let _ = state
        .db
        .upsert_identity_key(user.user_id, body.identity_key, body.registration_id)
        .await?;

    let _ = state
        .db
        .upsert_signed_prekey(
            user.user_id,
            body.signed_prekey.key_id,
            body.signed_prekey.public_key,
            body.signed_prekey.signature,
        )
        .await?;

    let ot_keys = body
        .one_time_prekeys
        .into_iter()
        .map(|k| (k.key_id, k.public_key))
        .collect();

    state
        .db
        .upload_one_time_prekeys(user.user_id, ot_keys)
        .await?;

    Ok(())
}

#[instrument(skip(state))]
pub async fn get_key_count(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<KeyCountRespDto>, AppError> {
    info!("Getting prekey bundle count for user {}", user.user_id);

    let count = state.db.get_prekey_bundle_counts(user.user_id).await?;

    Ok(Json(KeyCountRespDto { count }))
}

#[instrument(skip(state))]
pub async fn get_prekey_bundle(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<Json<PreKeyBundleRespDto>, AppError> {
    info!("Getting prekey bundle for user: {}", username);

    let user = match state.db.get_user_by_username(&username).await? {
        Some(user) => user,
        None => {
            warn!("User not found: {}", username);
            return Err(AppError::UserNotFound);
        }
    };

    let user_id = user.id;

    let identity_key = match state.db.get_identity_key(user_id).await? {
        Some(key) => key,
        None => {
            warn!("User {} has no identity key", user_id);
            return Err(AppError::UserHasNoKeys);
        }
    };

    let signed_prekey = match state.db.get_signed_prekey(user_id).await? {
        Some(key) => key,
        None => {
            warn!("User {} has no signed prekey", user_id);
            return Err(AppError::UserHasNoKeys);
        }
    };

    let one_time_prekey = state.db.consume_one_time_prekey(user_id).await?;

    Ok(Json(PreKeyBundleRespDto {
        identity_key: identity_key.identity_key,
        registration_id: identity_key.registration_id,
        signed_prekey: SignedPreKeyDto {
            key_id: signed_prekey.key_id,
            public_key: signed_prekey.public_key,
            signature: signed_prekey.signature,
        },
        one_time_prekey: one_time_prekey.map(|k| OneTimePreKeyDto {
            key_id: k.key_id,
            public_key: k.public_key,
        }),
    }))
}
