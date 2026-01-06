use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

use crate::errors::error_codes;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiErrorItem {
    pub code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl ApiErrorItem {
    pub fn new(code: &'static str, details: impl Into<Option<Value>>) -> Self {
        Self {
            code,
            details: details.into(),
        }
    }
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),

    #[error("Internal server error")]
    Internal,

    #[error("Validation failed")]
    Validation(Vec<ApiErrorItem>),

    // Auth
    #[error("Wrong credentials")]
    WrongCredentials,
    #[error("Session expired")]
    SessionExpired,
    #[error("Invalid token")]
    InvalidToken,

    // User
    #[error("Username already exists")]
    UsernameAlreadyExists,
    #[error("User not found")]
    UserNotFound,
    #[error("User has no keys")]
    UserHasNoKeys,

    // Room
    #[error("Room not found")]
    RoomNotFound,
    #[error("Already room member")]
    AlreadyRoomMember,
    #[error("Target already room member")]
    TargetAlreadyRoomMember,
    #[error("Not room member")]
    NotRoomMember,
    #[error("Target not room member")]
    TargetNotRoomMember,
    #[error("Not room admin")]
    NotRoomAdmin,

    // Invitation
    #[error("Invitation not found")]
    InvitationNotFound,
    #[error("No pending invitation")]
    NoPendingInvitation,
    #[error("Already invited")]
    AlreadyInvited,

    // Message
    #[error("Message not found")]
    MessageNotFound,
    #[error("Not message author")]
    NotMessageAuthor,

    // File
    #[error("File not found")]
    FileNotFound,
    #[error("Exceeding file limit")]
    ExceedingFileLimit,

    // General
    #[error("Invalid request format")]
    InvalidRequestFormat,
}

impl AppError {
    pub fn to_api_errors(&self) -> Vec<ApiErrorItem> {
        match self {
            AppError::Db(_) | AppError::Internal => {
                vec![ApiErrorItem::new(error_codes::INTERNAL_SERVER_ERROR, None)]
            }
            AppError::Validation(errors) => errors.clone(),
            AppError::InvalidRequestFormat => {
                vec![ApiErrorItem::new(error_codes::INVALID_REQUEST_FORMAT, None)]
            }
            AppError::UserHasNoKeys => {
                vec![ApiErrorItem::new(error_codes::USER_HAS_NO_KEYS, None)]
            }
            AppError::WrongCredentials => {
                vec![ApiErrorItem::new(error_codes::WRONG_CREDENTIALS, None)]
            }
            AppError::SessionExpired => {
                vec![ApiErrorItem::new(error_codes::SESSION_EXPIRED, None)]
            }
            AppError::InvalidToken => {
                vec![ApiErrorItem::new(error_codes::INVALID_TOKEN, None)]
            }
            AppError::UserNotFound => {
                vec![ApiErrorItem::new(error_codes::USER_NOT_FOUND, None)]
            }
            AppError::RoomNotFound => {
                vec![ApiErrorItem::new(error_codes::ROOM_NOT_FOUND, None)]
            }
            AppError::InvitationNotFound => {
                vec![ApiErrorItem::new(error_codes::INVITATION_NOT_FOUND, None)]
            }
            AppError::MessageNotFound => {
                vec![ApiErrorItem::new(error_codes::MESSAGE_NOT_FOUND, None)]
            }
            AppError::NoPendingInvitation => {
                vec![ApiErrorItem::new(error_codes::NO_PENDING_INVITATION, None)]
            }
            AppError::UsernameAlreadyExists => {
                vec![ApiErrorItem::new(
                    error_codes::USERNAME_ALREADY_EXISTS,
                    None,
                )]
            }
            AppError::AlreadyRoomMember => {
                vec![ApiErrorItem::new(error_codes::ALREADY_ROOM_MEMBER, None)]
            }
            AppError::TargetAlreadyRoomMember => {
                vec![ApiErrorItem::new(
                    error_codes::TARGET_ALREADY_ROOM_MEMBER,
                    None,
                )]
            }
            AppError::AlreadyInvited => {
                vec![ApiErrorItem::new(error_codes::ALREADY_INVITED, None)]
            }
            AppError::NotRoomMember => {
                vec![ApiErrorItem::new(error_codes::NOT_ROOM_MEMBER, None)]
            }
            AppError::TargetNotRoomMember => {
                vec![ApiErrorItem::new(error_codes::TARGET_NOT_ROOM_MEMBER, None)]
            }
            AppError::NotRoomAdmin => {
                vec![ApiErrorItem::new(error_codes::NOT_ROOM_ADMIN, None)]
            }
            AppError::NotMessageAuthor => {
                vec![ApiErrorItem::new(error_codes::NOT_MESSAGE_AUTHOR, None)]
            }
            AppError::FileNotFound => {
                vec![ApiErrorItem::new(error_codes::FILE_NOT_FOUND, None)]
            }
            AppError::ExceedingFileLimit => {
                vec![ApiErrorItem::new(error_codes::FILE_LIMIT_EXCEEDED, None)]
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, errors) = match &self {
            // 500
            AppError::Db(e) => {
                tracing::error!("Database error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_api_errors())
            }
            AppError::Internal => {
                tracing::error!("Internal server error");
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_api_errors())
            }

            // 400 - Validation
            AppError::Validation(errors) => {
                tracing::debug!("Validation failed: {:?}", errors);
                (StatusCode::BAD_REQUEST, errors.clone())
            }
            AppError::InvalidRequestFormat => {
                tracing::debug!("Invalid request format");
                (StatusCode::BAD_REQUEST, self.to_api_errors())
            }
            AppError::UserHasNoKeys => {
                tracing::debug!("User has no keys");
                (StatusCode::BAD_REQUEST, self.to_api_errors())
            }

            // 401
            AppError::WrongCredentials => {
                tracing::warn!("Wrong credentials");
                (StatusCode::UNAUTHORIZED, self.to_api_errors())
            }
            AppError::SessionExpired => {
                tracing::debug!("Session expired");
                (StatusCode::UNAUTHORIZED, self.to_api_errors())
            }
            AppError::InvalidToken => {
                tracing::warn!("Invalid token");
                (StatusCode::UNAUTHORIZED, self.to_api_errors())
            }

            // 404
            AppError::UserNotFound => {
                tracing::debug!("User not found");
                (StatusCode::NOT_FOUND, self.to_api_errors())
            }
            AppError::RoomNotFound => {
                tracing::debug!("Room not found");
                (StatusCode::NOT_FOUND, self.to_api_errors())
            }
            AppError::InvitationNotFound => {
                tracing::debug!("Invitation not found");
                (StatusCode::NOT_FOUND, self.to_api_errors())
            }
            AppError::MessageNotFound => {
                tracing::debug!("Message not found");
                (StatusCode::NOT_FOUND, self.to_api_errors())
            }
            AppError::NoPendingInvitation => {
                tracing::debug!("No pending invitation");
                (StatusCode::NOT_FOUND, self.to_api_errors())
            }

            // 409
            AppError::UsernameAlreadyExists => {
                tracing::debug!("Username already exists");
                (StatusCode::CONFLICT, self.to_api_errors())
            }
            AppError::AlreadyRoomMember => {
                tracing::debug!("Already room member");
                (StatusCode::CONFLICT, self.to_api_errors())
            }
            AppError::TargetAlreadyRoomMember => {
                tracing::debug!("Target already room member");
                (StatusCode::CONFLICT, self.to_api_errors())
            }
            AppError::AlreadyInvited => {
                tracing::debug!("Already invited");
                (StatusCode::CONFLICT, self.to_api_errors())
            }

            // 403
            AppError::NotRoomMember => {
                tracing::warn!("Not room member");
                (StatusCode::FORBIDDEN, self.to_api_errors())
            }
            AppError::TargetNotRoomMember => {
                tracing::warn!("Target not room member");
                (StatusCode::FORBIDDEN, self.to_api_errors())
            }
            AppError::NotRoomAdmin => {
                tracing::warn!("Not room admin");
                (StatusCode::FORBIDDEN, self.to_api_errors())
            }
            AppError::NotMessageAuthor => {
                tracing::warn!("Not message author");
                (StatusCode::FORBIDDEN, self.to_api_errors())
            }
            AppError::FileNotFound => {
                tracing::debug!("File not found");
                (StatusCode::NOT_FOUND, self.to_api_errors())
            }
            AppError::ExceedingFileLimit => {
                tracing::debug!("Exceeding file limit");
                (StatusCode::BAD_REQUEST, self.to_api_errors())
            }
        };

        (status, Json(json!({ "errors": errors }))).into_response()
    }
}
