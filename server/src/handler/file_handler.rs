use axum::{
    Json,
    extract::{Multipart, State},
    http::HeaderMap,
};
use tracing::{info, instrument};

use crate::{
    config::AppState,
    database::{
        files::FileRepository, room_members::RoomMemberRepository, user_messages::MessageRepository,
    },
    dtos::{GetFileReqDto, GetFileRespDto, UploadFileRespDto},
    errors::error::AppError,
    utils::{hash::hash_data, token::extract_and_verify_token},
};

const MAX_FILE_SIZE: usize = 50 * 1024 * 1024; // 50 MB

#[instrument(skip(state, headers, body))]
pub async fn upload_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut body: Multipart,
) -> Result<Json<UploadFileRespDto>, AppError> {
    info!("Uploading file");
    let (_, _, _) = extract_and_verify_token(&headers, state.config.jwt_secret.as_bytes())?;

    let mut encrypted_data: Option<Vec<u8>> = None;
    let mut encrypted_metadata: Option<Vec<u8>> = None;

    while let Some(field) = body
        .next_field()
        .await
        .map_err(|_| AppError::InvalidRequestFormat)?
    {
        let name = field.name().unwrap_or("").to_string();
        let data = field
            .bytes()
            .await
            .map_err(|_| AppError::InvalidRequestFormat)?;

        if data.len() > MAX_FILE_SIZE {
            return Err(AppError::InvalidRequestFormat);
        }

        match name.as_str() {
            "encrypted_data" => encrypted_data = Some(data.to_vec()),
            "encrypted_metadata" => encrypted_metadata = Some(data.to_vec()),
            _ => (),
        }
    }

    let encrypted_data = match encrypted_data {
        Some(d) => d,
        None => {
            return Err(AppError::ExceedingFileLimit);
        }
    };

    let size_in_bytes = encrypted_data.len() as i64;

    let file_hash = hash_data(&encrypted_data);

    let file = state
        .db
        .insert_file(encrypted_data, encrypted_metadata, size_in_bytes, file_hash)
        .await?;

    Ok(Json(UploadFileRespDto {
        file_id: file.id,
        size_in_bytes: file.size_in_bytes,
        uploaded_at: file.uploaded_at,
    }))
}

#[instrument(skip(state, headers, body))]
pub async fn get_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<GetFileReqDto>,
) -> Result<Json<GetFileRespDto>, AppError> {
    info!("Getting file");
    let (user_id, _, _) = extract_and_verify_token(&headers, state.config.jwt_secret.as_bytes())?;

    let message = match state.db.get_message_by_id(body.message_id).await? {
        Some(msg) => msg,
        None => return Err(AppError::MessageNotFound),
    };

    if !state.db.is_member(message.room_id, user_id).await? {
        return Err(AppError::NotRoomMember);
    }

    let file = match state.db.get_file(body.file_id).await? {
        Some(file) => file,
        None => return Err(AppError::FileNotFound),
    };

    Ok(Json(GetFileRespDto {
        file_id: file.id,
        encrypted_data: file.encrypted_data,
        encrypted_metadata: file.encrypted_metadata,
        size_in_bytes: file.size_in_bytes,
        uploaded_at: file.uploaded_at,
    }))
}
