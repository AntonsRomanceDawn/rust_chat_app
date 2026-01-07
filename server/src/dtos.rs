use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    database::models::{InvitationStatus, MessageStatus, MessageType, UserRole},
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

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadFileRespDto {
    pub file_id: Uuid,
    pub size_in_bytes: i64,
    pub uploaded_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetFileReqDto {
    pub file_id: Uuid,
    pub message_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetFileRespDto {
    pub file_id: Uuid,
    pub encrypted_data: Vec<u8>,
    pub encrypted_metadata: Option<Vec<u8>>,
    pub size_in_bytes: i64,
    pub uploaded_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadKeysReqDto {
    pub identity_key: String,
    pub registration_id: i32,
    pub signed_prekey: SignedPreKeyDto,
    pub one_time_prekeys: Vec<OneTimePreKeyDto>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PreKeyBundleRespDto {
    pub identity_key: String,
    pub registration_id: i32,
    pub signed_prekey: SignedPreKeyDto,
    pub one_time_prekey: Option<OneTimePreKeyDto>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyCountRespDto {
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignedPreKeyDto {
    pub key_id: i32,
    pub public_key: String,
    pub signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OneTimePreKeyDto {
    pub key_id: i32,
    pub public_key: String,
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
        message_type: Option<MessageType>,
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
        message_type: MessageType,
        created_at: DateTime<Utc>,
    },
    MessageReceived {
        message_id: Uuid,
        room_id: Uuid,
        room_name: String,
        author_username: Option<String>,
        content: String,
        message_type: MessageType,
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
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SystemMessageContent {
    Joined { username: String },
    Left { username: String },
    Kicked { username: String, by: String },
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
    pub author_username: Option<String>,
    pub content: String,
    pub message_type: MessageType,
    pub message_status: MessageStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RoomInfo {
    pub room_id: Uuid,
    pub room_name: String,
    pub last_message: Option<MessageInfo>,
    pub unread_count: i32,
}
