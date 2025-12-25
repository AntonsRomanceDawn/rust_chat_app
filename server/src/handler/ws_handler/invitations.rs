use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::{
    config::AppState,
    database::{
        invitations::InvitationRepository, models::InvitationStatus,
        room_members::RoomMemberRepository, rooms::RoomRepository, users::UserRepository,
    },
    dtos::{InvitationInfo, ServerResp},
    errors::error::AppError,
};

use super::utils::{send_error, send_event};

#[instrument(skip(state), fields(user_id = %user_id))]
pub async fn invite_response(state: &&AppState, user_id: Uuid, room_id: Uuid, username: String) {
    info!(
        "User {} is attempting to invite {} to room {}",
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
            error!("Failed to get room by id: {}: {:?}", room_id, e);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    let invitee = match state.db.get_user_by_username(&username).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            warn!("Invite failed: User {} not found", username);
            let _ = send_error(state, user_id, AppError::UserNotFound);
            return;
        }
        Err(e) => {
            error!("Failed to get user by username: {}: {:?}", username, e);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    let _ = match state.db.is_member(room_id, invitee.id).await {
        Ok(true) => {
            warn!(
                "Invite failed: User {} is already a member of room {}",
                username, room_id
            );
            let _ = send_error(state, user_id, AppError::TargetAlreadyRoomMember);
            return;
        }
        Err(e) => {
            error!("Database error checking membership: {:?}", e);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
        _ => {}
    };

    let inviter = match state.db.get_user_by_id(user_id).await {
        Ok(Some(user)) => user,
        _ => {
            error!("Failed to get inviter user {}", user_id);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    let _ = match state.db.is_member(room_id, user_id).await {
        Ok(false) => {
            warn!(
                "Invite failed: Inviter {} is not a member of room {}",
                user_id, room_id
            );
            let _ = send_error(state, user_id, AppError::NotRoomMember);
            return;
        }
        Err(e) => {
            error!("Database error checking inviter membership: {:?}", e);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
        _ => {}
    };

    let _ = match state
        .db
        .create_invitation(
            room_id,
            room_name.clone(),
            invitee.id,
            invitee.username.clone(),
            inviter.id,
            inviter.username.clone(),
        )
        .await
    {
        Ok(Some(invitation)) => {
            info!(
                "Invitation created: {} invited {} to {}",
                inviter.username, invitee.username, room_name
            );
            let _ = send_event(
                state,
                user_id,
                ServerResp::InvitationSent {
                    invitation_id: invitation.id,
                    room_id: invitation.room_id,
                    room_name: invitation.room_name.clone(),
                    invitee_username: invitation.invitee_username,
                },
            );
            let _ = send_event(
                state,
                invitation.invitee_id,
                ServerResp::InvitationReceived {
                    invitation_id: invitation.id,
                    room_id: invitation.room_id,
                    room_name: invitation.room_name,
                    inviter_username: invitation.inviter_username,
                },
            );
        }
        Ok(None) => {
            debug!(
                "Invite failed: User {} has already been invited to room {}",
                username, room_id
            );
            let _ = send_error(state, user_id, AppError::AlreadyInvited);
        }
        Err(e) => {
            error!("Database error creating invitation: {:?}", e);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };
}

#[instrument(skip(state), fields(user_id = %user_id))]
pub async fn decline_invitation_response(state: &&AppState, user_id: Uuid, invitation_id: Uuid) {
    info!(
        "User {} is attempting to decline invitation {}",
        user_id, invitation_id
    );
    let invitation = match state.db.get_invitation_by_id(invitation_id).await {
        Ok(Some(invitation)) => invitation,
        Ok(None) => {
            warn!("Invitation not found: {}", invitation_id);
            let _ = send_error(state, user_id, AppError::InvitationNotFound);
            return;
        }
        Err(e) => {
            error!(
                "Database error getting invitation by id {}: {:?}",
                invitation_id, e
            );
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    let _ = match state.db.get_room_by_id(invitation.room_id).await {
        Ok(Some(_)) => {}
        _ => {
            error!("Failed to get room by id: {}", invitation.room_id);
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };

    let _ = match state
        .db
        .update_invitation_status(invitation_id, InvitationStatus::Declined)
        .await
    {
        Ok(Some(invitation)) => {
            info!("User {} declined invitation {}", user_id, invitation_id);
            let _ = send_event(
                state,
                user_id,
                ServerResp::InvitationDeclined { invitation_id },
            );

            let _ = send_event(
                state,
                invitation.inviter_id,
                ServerResp::InviteeDeclined {
                    invitation_id,
                    room_id: invitation.room_id,
                    room_name: invitation.room_name,
                    invitee_username: invitation.invitee_username,
                },
            );
        }
        Ok(None) => {
            warn!(
                "No pending invitation found to decline for invitation id {}",
                invitation_id
            );
            let _ = send_error(state, user_id, AppError::NoPendingInvitation);
            return;
        }
        Err(e) => {
            error!(
                "Database error updating invitation status for id {}: {:?}",
                invitation_id, e
            );
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };
}

#[instrument(skip(state), fields(user_id = %user_id))]
pub async fn get_pending_invitations_response(state: &&AppState, user_id: Uuid) {
    info!("User {} is requesting their pending invitations", user_id);
    let _ = match state.db.get_pending_invitations_for_user(user_id).await {
        Ok(invitations) => {
            let invitation_infos = invitations
                .into_iter()
                .map(|inv| InvitationInfo {
                    invitation_id: inv.id,
                    room_id: inv.room_id,
                    room_name: inv.room_name,
                    status: inv.status,
                    inviter_username: inv.inviter_username,
                    created_at: inv.created_at,
                })
                .collect::<Vec<InvitationInfo>>();
            info!(
                "Sending {} pending invitations to user {}",
                invitation_infos.len(),
                user_id
            );
            let _ = send_event(
                state,
                user_id,
                ServerResp::PendingInvitations {
                    pending_invitations: invitation_infos,
                },
            );
        }
        Err(e) => {
            error!(
                "Database error getting pending invitations for user {}: {:?}",
                user_id, e
            );
            let _ = send_error(state, user_id, AppError::Internal);
            return;
        }
    };
}
