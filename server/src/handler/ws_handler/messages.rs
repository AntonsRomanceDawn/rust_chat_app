use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::{
    config::AppState,
    database::{
        models::MessageType, room_members::RoomMemberRepository, rooms::RoomRepository,
        user_messages::MessageRepository, users::UserRepository,
    },
    dtos::{MessageInfo, ServerResp},
    errors::error::AppError,
};

use super::utils::{send_error, send_event};

#[instrument(skip(state), fields(user_id = %user_id))]
pub async fn send_message_response(
    state: &&AppState,
    user_id: Uuid,
    room_id: Uuid,
    content: String,
    message_type: Option<MessageType>,
) {
    info!("User {} is sending message to room {}", user_id, room_id);
    let message_type = message_type.unwrap_or(MessageType::Text);
    let room_name = match state.db.get_room_by_id(room_id).await {
        Ok(Some(room)) => room.name,
        Ok(None) => {
            warn!("Room not found: {}", room_id);
            let _ = send_error(state, user_id, AppError::RoomNotFound);
            return;
        }
        Err(e) => {
            error!("Database error getting room by id {}: {:?}", room_id, e);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    let _ = match state.db.is_member(room_id, user_id).await {
        Ok(false) => {
            warn!("User {} is not a member of room {}", user_id, room_id);
            let _ = send_error(state, user_id, AppError::NotRoomMember);
            return;
        }
        Err(e) => {
            error!(
                "Database error checking membership for user {} in room {}: {:?}",
                user_id, room_id, e
            );
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
        _ => {}
    };

    let author = match state.db.get_user_by_id(user_id).await {
        Ok(Some(user)) => user,
        _ => {
            error!("Failed to get author user {}", user_id);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    let message = match state
        .db
        .insert_message(
            room_id,
            room_name,
            Some(author.id),
            Some(author.username),
            &content,
            message_type,
        )
        .await
    {
        Ok(message) => message,
        Err(e) => {
            error!(
                "Database error inserting message in room {}: {:?}",
                room_id, e
            );
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    let member_ids = match state.db.get_members(room_id).await {
        Ok(members) => members
            .into_iter()
            .map(|m| m.user_id)
            .collect::<Vec<Uuid>>(),
        Err(e) => {
            error!(
                "Database error getting members for room {}: {:?}",
                room_id, e
            );
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    info!(
        "Broadcasting message {} to room {} members",
        message.id, room_id
    );

    let event = ServerResp::MessageReceived {
        message_id: message.id,
        room_id: message.room_id,
        room_name: message.room_name.clone(),
        author_username: message.author_username,
        content: message.content.clone(),
        message_type: message.message_type,
        created_at: message.created_at,
    };

    for member_id in member_ids {
        if member_id == user_id {
            continue;
        }
        let _ = state
            .db
            .increment_unread_count(room_id, member_id)
            .await
            .map_err(|e| {
                error!(
                    "Database error incrementing unread count for user {} in room {}: {:?}",
                    member_id, room_id, e
                );
            });
        let _ = send_event(state, member_id, event.clone());
    }

    let _ = send_event(
        state,
        user_id,
        ServerResp::MessageSent {
            message_id: message.id,
            room_id: message.room_id,
            room_name: message.room_name,
            content: message.content,
            message_type: message.message_type,
            created_at: message.created_at,
        },
    );
}

#[instrument(skip(state), fields(user_id = %user_id))]
pub async fn edit_message_response(
    state: &&AppState,
    user_id: Uuid,
    message_id: Uuid,
    new_content: String,
) {
    info!("User {} is editing message {}", user_id, message_id);
    let message = match state.db.get_message_by_id(message_id).await {
        Ok(Some(message)) => message,
        Ok(None) => {
            warn!("Message not found: {}", message_id);
            let _ = send_error(state, user_id, AppError::MessageNotFound);
            return;
        }
        Err(e) => {
            error!(
                "Database error getting message by id {}: {:?}",
                message_id, e
            );
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    if message.author_id != Some(user_id) {
        warn!(
            "User {} is not the author of message {}",
            user_id, message_id
        );
        let _ = send_error(state, user_id, AppError::NotMessageAuthor);
        return;
    }

    let _ = match state
        .db
        .update_message_content(message_id, &new_content)
        .await
    {
        Ok(Some(updated_message)) => {
            info!("User {} edited message {}", user_id, message_id);
            let event = ServerResp::MessageEdited {
                message_id: updated_message.id,
                new_content: updated_message.content.clone(),
            };
            if let Ok(members) = state.db.get_members(updated_message.room_id).await {
                for member in members {
                    let _ = send_event(state, member.user_id, event.clone());
                }
            } else {
                error!(
                    "Database error getting members for room {}",
                    updated_message.room_id
                );
                let _ = send_error(state, user_id, AppError::Internal);
                return;
            }
        }
        _ => {
            error!(
                "Database error updating message content for message {}",
                message_id
            );
            let _ = send_error(state, user_id, AppError::Internal);
        }
    };
}

#[instrument(skip(state), fields(user_id = %user_id))]
pub async fn delete_message_response(state: &&AppState, user_id: Uuid, message_id: Uuid) {
    info!("User {} is deleting message {}", user_id, message_id);
    let _ = match state.db.get_message_by_id(message_id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            warn!("Message not found: {}", message_id);
            let _ = send_error(state, user_id, AppError::MessageNotFound);
            return;
        }
        Err(e) => {
            error!(
                "Database error getting message by id {}: {:?}",
                message_id, e
            );
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    let _ = match state.db.delete_message(message_id).await {
        Ok(Some(message)) => {
            info!("User {} deleted message {}", user_id, message_id);
            let event = ServerResp::MessageDeleted { message_id };
            if let Ok(members) = state.db.get_members(message.room_id).await {
                for member in members {
                    let _ = send_event(state, member.user_id, event.clone());
                }
            } else {
                error!(
                    "Database error getting members for room {}",
                    message.room_id
                );
                let _ = send_error(state, user_id, AppError::Internal);
                return;
            }
        }
        _ => {
            error!("Database error deleting message {}", message_id);
            let _ = send_error(state, user_id, AppError::Internal);
        }
    };
}

#[instrument(skip(state), fields(user_id = %user_id))]
pub async fn get_messages_response(
    state: &&AppState,
    user_id: Uuid,
    room_id: Uuid,
    limit: i64,
    offset: i64,
) {
    info!(
        "User {} is requesting messages for room {}",
        user_id, room_id
    );
    let room_name = match state.db.get_room_by_id(room_id).await {
        Ok(Some(room)) => room.name,
        Ok(None) => {
            warn!("Room not found: {}", room_id);
            let _ = send_error(state, user_id, AppError::RoomNotFound);
            return;
        }
        Err(e) => {
            error!("Database error getting room by id {}: {:?}", room_id, e);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    let _ = match state
        .db
        .get_room_messages(room_id, user_id, limit, offset)
        .await
    {
        Ok(messages) => {
            if let Err(e) = state.db.reset_last_read_and_count(room_id, user_id).await {
                error!(
                    "Failed to update last_read_at for user {} in room {}: {:?}",
                    user_id, room_id, e
                );
            }

            let mut message_infos = Vec::new();
            for msg in messages {
                message_infos.push(MessageInfo {
                    message_id: msg.id,
                    author_username: msg.author_username,
                    content: msg.content,
                    message_type: msg.message_type,
                    message_status: msg.status,
                    created_at: msg.created_at,
                });
            }
            info!(
                "Sending {} messages to user {} for room {}",
                message_infos.len(),
                user_id,
                room_id
            );
            debug!("Messages: {:?}", message_infos);
            let _ = send_event(
                state,
                user_id,
                ServerResp::MessageHistory {
                    room_id,
                    room_name,
                    messages: message_infos,
                },
            );
        }
        Err(_) => {
            let _ = send_error(state, user_id, AppError::Internal);
        }
    };
}
