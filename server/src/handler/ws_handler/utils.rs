use crate::config::AppState;
use crate::database::{
    models::MessageType, room_members::RoomMemberRepository, user_messages::MessageRepository,
};
use crate::dtos::{ServerResp, SystemMessageContent};
use crate::errors::error::AppError;
use tracing::{debug, error};
use uuid::Uuid;

pub fn send_event(state: &AppState, user_id: Uuid, event: ServerResp) {
    if let Some(sender) = state.channels.get(&user_id) {
        sender.send(event).ok();
    }
}

pub fn send_error(state: &AppState, user_id: Uuid, error: AppError) {
    send_event(
        state,
        user_id,
        ServerResp::Error {
            errors: error.to_api_errors(),
        },
    );
}

use crate::database::models::UserMessage;

pub async fn create_and_broadcast_system_message(
    state: &AppState,
    room_id: Uuid,
    room_name: String,
    content: SystemMessageContent,
) -> Result<UserMessage, AppError> {
    let content_str = serde_json::to_string(&content).map_err(|e| {
        error!("Failed to serialize system message: {:?}", e);
        AppError::Internal
    })?;

    match state
        .db
        .insert_message(
            room_id,
            room_name.clone(),
            None,
            None,
            &content_str,
            MessageType::System,
        )
        .await
    {
        Ok(message) => {
            if let Ok(members) = state.db.get_members(room_id).await {
                let event = ServerResp::MessageReceived {
                    message_id: message.id,
                    room_id: message.room_id,
                    room_name: message.room_name.clone(),
                    author_username: message.author_username.clone(),
                    content: message.content.clone(),
                    message_type: message.message_type,
                    created_at: message.created_at,
                };
                debug!("Broadcasting system message event: {:?}", event);
                for member in members {
                    let _ = send_event(state, member.user_id, event.clone());
                }
            }
            Ok(message)
        }
        Err(e) => {
            error!("Failed to insert system message: {:?}", e);
            Err(AppError::Internal)
        }
    }
}
