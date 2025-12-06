use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    database::models::{InvitationStatus, UserRole},
    errors::error::ApiErrorItem,
    utils::validation::{validate_confirm_password, validate_password, validate_username},
};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RegisterReqDto {
    pub username: String,
    pub password: String,
    pub confirm_password: String,
}

impl RegisterReqDto {
    pub fn validate(&self) -> Result<(), Vec<ApiErrorItem>> {
        let mut errors = Vec::new();

        errors.extend(validate_username(&self.username));
        errors.extend(validate_password(&self.password));
        errors.extend(validate_confirm_password(
            &self.password,
            &self.confirm_password,
        ));

        match errors.is_empty() {
            true => Ok(()),
            false => Err(errors),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRespDto {
    pub id: Uuid,
    pub username: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LoginReqDto {
    pub username: String,
    pub password: String,
}

impl LoginReqDto {
    pub fn validate(&self) -> Result<(), Vec<ApiErrorItem>> {
        let mut errors = Vec::new();
        errors.extend(validate_username(&self.username));
        errors.extend(validate_password(&self.password));

        match errors.is_empty() {
            true => Ok(()),
            false => Err(errors),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRespDto {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshTokenReqDto {
    pub refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshTokenRespDto {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Deserialize, Debug)]
pub struct WsParams {
    pub token: String,
}

// WebSocket request/response DTOs
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientReq {
    CreateRoom {
        name: String,
    },
    JoinRoom {
        invitation_id: Uuid,
    },
    LeaveRoom {
        room_id: Uuid,
    },
    UpdateRoom {
        room_id: Uuid,
        name: String,
    },
    DeleteRoom {
        room_id: Uuid,
    },
    GetRoomInfo {
        room_id: Uuid,
    },
    GetRoomsInfo,
    Invite {
        room_id: Uuid,
        username: String,
    },
    DeclineInvitation {
        invitation_id: Uuid,
    },
    GetPendingInvitations,
    SendMessage {
        room_id: Uuid,
        content: String,
    },
    EditMessage {
        message_id: Uuid,
        new_content: String,
    },
    DeleteMessage {
        message_id: Uuid,
    },
    GetMessages {
        room_id: Uuid,
        limit: i64,
        offset: i64,
    },
    DeleteAccount,
    KickMember {
        room_id: Uuid,
        username: String,
    },
    SearchUsers {
        query: String,
    },
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerResp {
    RoomCreated {
        room_id: Uuid,
        room_name: String,
        created_at: DateTime<Utc>,
    },
    RoomJoined {
        invitation_id: Uuid,
        room_id: Uuid,
        room_name: String,
        admin_username: String,
        creator_username: String,
        created_at: DateTime<Utc>,
        joined_at: DateTime<Utc>,
    },
    RoomLeft {
        room_id: Uuid,
        room_name: String,
    },
    RoomUpdated {
        room_id: Uuid,
        room_name: String,
    },
    RoomDeleted {
        room_id: Uuid,
        room_name: String,
    },
    RoomInfo {
        room_id: Uuid,
        room_name: String,
        admin_username: String,
        creator_username: String,
        members: Vec<MemberInfo>,
        created_at: DateTime<Utc>,
    },
    RoomsInfo {
        rooms: Vec<RoomInfo>,
    },
    InvitationReceived {
        invitation_id: Uuid,
        room_id: Uuid,
        room_name: String,
        inviter_username: String,
    },
    InvitationSent {
        invitation_id: Uuid,
        room_id: Uuid,
        room_name: String,
        invitee_username: String,
    },
    InvitationDeclined {
        invitation_id: Uuid,
    },
    InviteeDeclined {
        invitation_id: Uuid,
        room_id: Uuid,
        room_name: String,
        invitee_username: String,
    },
    PendingInvitations {
        pending_invitations: Vec<InvitationInfo>,
    },
    MessageSent {
        message_id: Uuid,
        room_id: Uuid,
        room_name: String,
        content: String,
        created_at: DateTime<Utc>,
    },
    MessageReceived {
        message_id: Uuid,
        room_id: Uuid,
        room_name: String,
        author_username: String,
        content: String,
        created_at: DateTime<Utc>,
    },
    MessageEdited {
        message_id: Uuid,
        new_content: String,
    },
    MessageDeleted {
        message_id: Uuid,
    },
    MessageHistory {
        room_id: Uuid,
        room_name: String,
        messages: Vec<MessageInfo>,
    },
    AccountDeleted {
        user_id: Uuid,
    },
    MemberKicked {
        room_id: Uuid,
        room_name: String,
        username: String,
    },
    MemberJoined {
        room_id: Uuid,
        room_name: String,
        username: String,
        joined_at: DateTime<Utc>,
    },
    UsersFound {
        users: Vec<UserInfo>,
    },
    Error {
        errors: Vec<ApiErrorItem>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserInfo {
    pub username: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InvitationInfo {
    pub invitation_id: Uuid,
    pub room_id: Uuid,
    pub room_name: String,
    pub status: InvitationStatus,
    pub inviter_username: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MemberInfo {
    pub username: String,
    pub joined_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MessageInfo {
    pub message_id: Uuid,
    pub author_username: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RoomInfo {
    pub room_id: Uuid,
    pub room_name: String,
    pub unread_count: i64,
}
