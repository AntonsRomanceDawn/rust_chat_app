use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum UserRole {
    Admin,
    User,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum MessageType {
    Text,
    File,
    System,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum MessageStatus {
    Sent,
    Edited,
    Deleted,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, sqlx::Type, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Declined,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Room {
    pub id: Uuid,
    pub name: String,
    pub creator_id: Uuid,
    pub creator_username: String,
    pub admin_id: Uuid,
    pub admin_username: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct RoomMember {
    pub room_id: Uuid,
    pub room_name: String,
    pub user_id: Uuid,
    pub username: String,
    pub joined_at: DateTime<Utc>,
    pub last_read_at: DateTime<Utc>,
    pub unread_count: i32,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserMessage {
    pub id: Uuid,
    pub room_id: Uuid,
    pub room_name: String,
    pub author_id: Uuid,
    pub author_username: String,
    pub content: String,
    pub message_type: MessageType,
    pub status: MessageStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct FileRecord {
    pub id: Uuid,
    pub encrypted_data: Vec<u8>,
    pub encrypted_metadata: Option<Vec<u8>>,
    pub size_in_bytes: i64,
    pub file_hash: String,
    pub uploaded_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Invitation {
    pub id: Uuid,
    pub room_id: Uuid,
    pub room_name: String,
    pub invitee_id: Uuid,
    pub invitee_username: String,
    pub inviter_id: Uuid,
    pub inviter_username: String,
    pub status: InvitationStatus,
    pub created_at: DateTime<Utc>,
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                UserRole::Admin => "admin",
                UserRole::User => "user",
            }
        )
    }
}

impl From<String> for UserRole {
    fn from(value: String) -> Self {
        match value.as_str() {
            "admin" => UserRole::Admin,
            "user" => UserRole::User,
            _ => UserRole::User,
        }
    }
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MessageType::Text => "text",
                MessageType::File => "file",
                MessageType::System => "system",
            }
        )
    }
}

impl From<String> for MessageType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "text" => MessageType::Text,
            "file" => MessageType::File,
            "system" => MessageType::System,
            _ => MessageType::Text,
        }
    }
}

impl std::fmt::Display for MessageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageStatus::Sent => write!(f, "sent"),
            MessageStatus::Edited => write!(f, "edited"),
            MessageStatus::Deleted => write!(f, "deleted"),
        }
    }
}

impl From<String> for MessageStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "sent" => MessageStatus::Sent,
            "edited" => MessageStatus::Edited,
            "deleted" => MessageStatus::Deleted,
            _ => MessageStatus::Sent,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct IdentityKey {
    pub user_id: Uuid,
    pub identity_key: String,
    pub registration_id: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct SignedPreKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub key_id: i32,
    pub public_key: String,
    pub signature: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct OneTimePreKey {
    pub user_id: Uuid,
    pub key_id: i32,
    pub public_key: String,
    pub created_at: DateTime<Utc>,
}

impl std::fmt::Display for InvitationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                InvitationStatus::Pending => "pending",
                InvitationStatus::Accepted => "accepted",
                InvitationStatus::Declined => "declined",
            }
        )
    }
}

impl From<String> for InvitationStatus {
    fn from(value: String) -> Self {
        match value.as_str() {
            "pending" => InvitationStatus::Pending,
            "accepted" => InvitationStatus::Accepted,
            "declined" => InvitationStatus::Declined,
            _ => InvitationStatus::Declined,
        }
    }
}
