use chrono::Utc;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

use crate::{
    config::AppState,
    database::{
        invitations::InvitationRepository, room_members::RoomMemberRepository,
        rooms::RoomRepository, users::UserRepository,
    },
    dtos::{MemberInfo, MessageInfo, RoomInfo, ServerResp, SystemMessageContent},
    errors::error::AppError,
};

use crate::handler::ws_handler::utils::{
    create_and_broadcast_system_message, send_error, send_event,
};

#[instrument(skip(state), fields(user_id = %user_id))]
pub async fn create_room_response(state: &&AppState, user_id: Uuid, name: String) {
    info!("Creating room: {}", name);
    let username = match state.db.get_user_by_id(user_id).await {
        Ok(Some(user)) => user.username,
        _ => {
            error!("Failed to get user by id: {}", user_id);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    let _ = match state.db.create_room(&name, user_id, username).await {
        Ok(room) => {
            info!("Room created: {} with id {}", room.name, room.id);
            let _ = send_event(
                state,
                user_id,
                ServerResp::RoomCreated {
                    room_id: room.id,
                    room_name: room.name,
                    created_at: room.created_at,
                },
            );
        }
        Err(e) => {
            error!("Failed to create room: {:?}", e);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };
}

#[instrument(skip(state), fields(user_id = %user_id))]
pub async fn join_room_response(state: &&AppState, user_id: Uuid, invitation_id: Uuid) {
    info!(
        "User {} is attempting to join room with invitation {}",
        user_id, invitation_id
    );
    let room_id = match state.db.get_invitation_by_id(invitation_id).await {
        Ok(Some(invitation)) => invitation.room_id,
        Ok(None) => {
            warn!(
                "No pending invitation found for invitation id {}",
                invitation_id
            );
            let _ = send_error(state, user_id, AppError::NoPendingInvitation);
            return;
        }
        Err(e) => {
            error!("Failed to get invitation by id: {:?}", e);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    let room = match state.db.get_room_by_id(room_id).await {
        Ok(Some(room)) => room,
        _ => {
            error!("Failed to get room by id: {}", room_id);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    let members = match state.db.get_members(room_id).await {
        Ok(members) => members,
        Err(e) => {
            error!("Failed to get room members: {:?}", e);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    if members.iter().any(|m| m.user_id == user_id) {
        warn!("User {} is already a member of room {}", user_id, room_id);
        let _ = send_error(state, user_id, AppError::AlreadyRoomMember);
        return;
    }

    let admin_username = match members
        .iter()
        .find(|m| m.user_id == room.admin_id)
        .map(|m| m.username.clone())
    {
        Some(username) => username,
        None => {
            error!("Admin user not found in members for room {}", room_id);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    let creator_username = match state.db.get_user_by_id(room.creator_id).await {
        Ok(Some(user)) => user.username,
        _ => {
            error!("Failed to get creator user by id: {}", room.creator_id);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    let invitee_username = match state.db.get_user_by_id(user_id).await {
        Ok(Some(user)) => user.username,
        _ => {
            error!("Failed to get invitee user by id: {}", user_id);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    let now = Utc::now();

    let _ = state
        .db
        .consume_invitations_and_join_room(
            room_id,
            room.name.clone(),
            user_id,
            invitee_username.clone(),
            now,
        )
        .await
        .map_err(|e| {
            error!("Failed to consume invitation and join room: {:?}", e);
            let _ = send_error(state, user_id, AppError::Internal);
        });

    let _ = create_and_broadcast_system_message(
        state,
        room_id,
        room.name.clone(),
        SystemMessageContent::Joined {
            username: invitee_username.clone(),
        },
    )
    .await;

    let event = ServerResp::MemberJoined {
        room_id,
        room_name: room.name.clone(),
        username: invitee_username.clone(),
        joined_at: now,
    };

    info!("User {} joined room {}", user_id, room_id);
    for member_id in members.into_iter().map(|m| m.user_id) {
        let _ = send_event(state, member_id, event.clone());
    }

    let _ = send_event(
        state,
        user_id,
        ServerResp::RoomJoined {
            invitation_id,
            room_id: room.id,
            room_name: room.name,
            admin_username,
            creator_username,
            created_at: room.created_at,
            joined_at: now,
        },
    );
}

#[instrument(skip(state), fields(user_id = %user_id))]
pub async fn leave_room_response(state: &&AppState, user_id: Uuid, room_id: Uuid) {
    info!("User {} is attempting to leave room {}", user_id, room_id);
    let _ = match state.db.get_room_by_id(room_id).await {
        Ok(None) => {
            warn!("Room not found: {}", room_id);
            let _ = send_error(state, user_id, AppError::RoomNotFound);
            return;
        }
        Err(e) => {
            error!("Failed to get room by id: {}: {:?}", room_id, e);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
        Ok(Some(_)) => {}
    };

    let _ = match state.db.is_member(room_id, user_id).await {
        Ok(false) => {
            warn!("User {} is not a member of room {}", user_id, room_id);
            let _ = send_error(state, user_id, AppError::NotRoomMember);
            return;
        }
        Err(e) => {
            error!("Failed to check if user is a member of room: {:?}", e);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
        _ => {}
    };

    let username = match state.db.get_user_by_id(user_id).await {
        Ok(Some(u)) => u.username,
        _ => "Unknown".to_string(),
    };

    let _ = match state.db.leave_room(room_id, user_id).await {
        Ok(Some(room)) => {
            info!("User {} left room {}", user_id, room_id);

            let _ = create_and_broadcast_system_message(
                state,
                room_id,
                room.name.clone(),
                SystemMessageContent::Left { username },
            )
            .await;

            let _ = send_event(
                state,
                user_id,
                ServerResp::RoomLeft {
                    room_id: room.id,
                    room_name: room.name,
                },
            );
        }
        _ => {
            error!("Failed to leave room: {}", room_id);
            let _ = send_error(state, user_id, AppError::Internal);
        }
    };
}

#[instrument(skip(state), fields(user_id = %user_id))]
pub async fn update_room_response(state: &&AppState, user_id: Uuid, room_id: Uuid, name: String) {
    info!("User {} is attempting to update room {}", user_id, room_id);
    let _ = match state.db.get_room_by_id(room_id).await {
        Ok(None) => {
            warn!("Room not found: {}", room_id);
            let _ = send_error(state, user_id, AppError::RoomNotFound);
            return;
        }
        Err(e) => {
            error!("Failed to get room by id: {}: {:?}", room_id, e);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
        Ok(Some(_)) => {}
    };

    let _ = match state.db.is_admin(room_id, user_id).await {
        Ok(false) => {
            warn!("User {} is not an admin of room {}", user_id, room_id);
            let _ = send_error(state, user_id, AppError::NotRoomAdmin);
            return;
        }
        Err(e) => {
            error!("Failed to check if user is an admin of room: {:?}", e);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
        _ => {}
    };

    let _ = match state.db.update_room_name(room_id, &name).await {
        Ok(Some(room)) => {
            info!("User {} updated room {}", user_id, room_id);
            let event = ServerResp::RoomUpdated {
                room_id: room.id,
                room_name: room.name.clone(),
            };
            if let Ok(members) = state.db.get_members(room_id).await {
                for member in members {
                    let _ = send_event(state, member.user_id, event.clone());
                }
            } else {
                error!("Failed to get members of room: {}", room_id);
                let _ = send_error(state, user_id, AppError::Internal);
                return;
            }
        }
        _ => {
            let _ = send_error(state, user_id, AppError::Internal);
        }
    };
}

#[instrument(skip(state), fields(user_id = %user_id))]
pub async fn delete_room_response(state: &&AppState, user_id: Uuid, room_id: Uuid) {
    info!("User {} is attempting to delete room {}", user_id, room_id);
    let _ = match state.db.get_room_by_id(room_id).await {
        Ok(None) => {
            warn!("Room not found: {}", room_id);
            let _ = send_error(state, user_id, AppError::RoomNotFound);
            return;
        }
        Err(e) => {
            error!("Failed to get room by id: {}: {:?}", room_id, e);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
        Ok(Some(_)) => {}
    };

    let _ = match state.db.is_admin(room_id, user_id).await {
        Ok(false) => {
            warn!("User {} is not an admin of room {}", user_id, room_id);
            let _ = send_error(state, user_id, AppError::NotRoomAdmin);
            return;
        }
        Err(e) => {
            error!("Failed to check if user is an admin of room: {:?}", e);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
        _ => {}
    };

    let _ = match state.db.delete_room(room_id).await {
        Ok(Some(room)) => {
            info!("User {} deleted room {}", user_id, room_id);
            let event = ServerResp::RoomDeleted {
                room_id: room.id,
                room_name: room.name.clone(),
            };
            if let Ok(members) = state.db.get_members(room_id).await {
                for member in members {
                    let _ = send_event(state, member.user_id, event.clone());
                }
            } else {
                error!("Failed to get members of room: {}", room_id);
                let _ = send_error(state, user_id, AppError::Internal);
                return;
            }
        }
        _ => {
            error!("Failed to delete room: {}", room_id);
            let _ = send_error(state, user_id, AppError::Internal);
        }
    };
}

#[instrument(skip(state), fields(user_id = %user_id))]
pub async fn get_room_info_response(state: &&AppState, user_id: Uuid, room_id: Uuid) {
    info!("User {} is requesting info for room {}", user_id, room_id);
    let room = match state.db.get_room_by_id(room_id).await {
        Ok(Some(room)) => room,
        Ok(None) => {
            warn!("Room not found: {}", room_id);
            let _ = send_error(state, user_id, AppError::RoomNotFound);
            return;
        }
        Err(e) => {
            error!("Failed to get room by id: {}: {:?}", room_id, e);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    let members = match state.db.get_members(room_id).await {
        Ok(members) => members,
        Err(e) => {
            error!("Failed to get members of room: {}: {:?}", room_id, e);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    let admin_username = match members
        .iter()
        .find(|m| m.user_id == room.admin_id)
        .map(|m| m.username.clone())
    {
        Some(username) => username,
        None => {
            error!("Admin user not found in members for room: {}", room_id);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    let creator_username = match state.db.get_user_by_id(room.creator_id).await {
        Ok(Some(user)) => user.username,
        _ => {
            error!("Failed to get creator user by id: {}", room.creator_id);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    let members_info = members
        .into_iter()
        .map(|m| MemberInfo {
            username: m.username,
            joined_at: m.joined_at,
        })
        .collect::<Vec<MemberInfo>>();

    info!("Sending room info for room {}", room_id);
    let _ = send_event(
        state,
        user_id,
        ServerResp::RoomInfo {
            room_id: room.id,
            room_name: room.name,
            admin_username,
            creator_username,
            members: members_info,
            created_at: room.created_at,
        },
    );
}

#[instrument(skip(state), fields(user_id = %user_id))]
pub async fn get_rooms_info_response(state: &&AppState, user_id: Uuid) {
    info!("User {} is requesting info for all their rooms", user_id);
    let _ = match state.db.get_rooms_info_for_user(user_id).await {
        Ok(rooms) => {
            info!("Sending rooms info to user {}", user_id);
            let rooms_info = rooms
                .into_iter()
                .map(|(member, last_message)| {
                    let last_message = last_message.map(|msg| MessageInfo {
                        message_id: msg.id,
                        author_username: msg.author_username,
                        content: msg.content,
                        message_type: msg.message_type,
                        message_status: msg.status,
                        created_at: msg.created_at,
                    });

                    RoomInfo {
                        room_id: member.room_id,
                        room_name: member.room_name,
                        last_message,
                        unread_count: member.unread_count,
                    }
                })
                .collect::<Vec<RoomInfo>>();
            let _ = send_event(state, user_id, ServerResp::RoomsInfo { rooms: rooms_info });
        }
        Err(e) => {
            error!("Failed to get user rooms for user {}: {:?}", user_id, e);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };
}
