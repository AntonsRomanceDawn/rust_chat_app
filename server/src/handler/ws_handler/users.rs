use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use crate::{
    config::AppState,
    database::{room_members::RoomMemberRepository, rooms::RoomRepository, users::UserRepository},
    dtos::{ServerResp, SystemMessageContent, UserInfo},
    errors::error::AppError,
};

use crate::handler::ws_handler::utils::{
    create_and_broadcast_system_message, send_error, send_event,
};

#[instrument(skip(state), fields(user_id = %user_id))]
pub async fn delete_account_response(state: &&AppState, user_id: Uuid) {
    info!("User {} is attempting to delete their account", user_id);
    let _ = match state.db.delete_user(user_id).await {
        Ok(Some(user)) => {
            info!("User {} deleted their account", user_id);
            let _ = send_event(
                state,
                user_id,
                ServerResp::AccountDeleted { user_id: user.id },
            );
        }
        _ => {
            error!("Failed to delete user account for user {}", user_id);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };
}
#[instrument(skip(state), fields(user_id = %user_id))]
pub async fn kick_member_response(
    state: &&AppState,
    user_id: Uuid,
    room_id: Uuid,
    username: String,
) {
    info!(
        "User {} is attempting to kick {} from room {}",
        user_id, username, room_id
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

    let _ = match state.db.is_admin(room_id, user_id).await {
        Ok(false) => {
            warn!("User {} is not an admin of room {}", user_id, room_id);
            let _ = send_error(state, user_id, AppError::NotRoomAdmin);
            return;
        }
        Err(e) => {
            error!(
                "Database error checking admin status for user {} in room {}: {:?}",
                user_id, room_id, e
            );
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
        _ => {}
    };

    let member = match state.db.get_user_by_username(&username).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            warn!("User not found: {}", username);
            let _ = send_error(state, user_id, AppError::UserNotFound);
            return;
        }
        Err(e) => {
            error!(
                "Database error getting user by username {}: {:?}",
                username, e
            );
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    let _ = match state.db.remove_member(room_id, member.id).await {
        Ok(Some(member)) => {
            info!("User {} was kicked from room {}", member.user_id, room_id);

            let admin_username = match state.db.get_user_by_id(user_id).await {
                Ok(Some(u)) => u.username,
                _ => "Admin".to_string(),
            };

            let sys_msg = create_and_broadcast_system_message(
                state,
                room_id,
                member.room_name.clone(),
                SystemMessageContent::Kicked {
                    username: member.username.clone(),
                    by: admin_username,
                },
            )
            .await;

            if let Ok(message) = sys_msg {
                // Also send the system message to the kicked user so they see it
                let event = ServerResp::MessageReceived {
                    message_id: message.id,
                    room_id: message.room_id,
                    room_name: message.room_name,
                    author_username: message.author_username,
                    content: message.content,
                    message_type: message.message_type,
                    created_at: message.created_at,
                };
                let _ = send_event(state, member.user_id, event);
            }

            if let Ok(members) = state.db.get_members(room_id).await {
                let event = ServerResp::MemberKicked {
                    room_id: member.room_id,
                    room_name: member.room_name,
                    username: member.username,
                };
                for m in members {
                    let _ = send_event(state, m.user_id, event.clone());
                }
            } else {
                error!("Database error getting members for room {}", room_id);
                let _ = send_error(state, user_id, AppError::Internal);
                return;
            }
            let event = ServerResp::MemberKicked {
                room_id,
                room_name,
                username: username.clone(),
            };
            let _ = send_event(state, member.user_id, event);
        }
        Ok(None) => {
            warn!(
                "User {} is not a member of room {}",
                member.username, room_id
            );
            let _ = send_error(state, user_id, AppError::TargetNotRoomMember);
            return;
        }
        Err(e) => {
            error!(
                "Database error removing member {} from room {}: {:?}",
                member.username, room_id, e
            );
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };
}

#[instrument(skip(state), fields(user_id = %user_id))]
pub async fn search_users_response(state: &&AppState, user_id: Uuid, query: String) {
    info!(
        "User {} is searching for users with query '{}'",
        user_id, query
    );
    let _ = match state.db.search_users(&query).await {
        Ok(users) => {
            info!(
                "Found {} users matching query '{}' for user {}",
                users.len(),
                query,
                user_id
            );
            let user_infos = users
                .into_iter()
                .map(|u| UserInfo {
                    username: u.username,
                    created_at: u.created_at,
                })
                .collect::<Vec<UserInfo>>();
            let _ = send_event(state, user_id, ServerResp::UsersFound { users: user_infos });
        }
        Err(e) => {
            error!(
                "Database error searching users with query '{}': {:?}",
                query, e
            );
            let _ = send_error(state, user_id, AppError::Internal);
        }
    };
}
