use async_trait::async_trait;
use chrono::Utc;
use tracing::instrument;
use uuid::Uuid;

use crate::database::{
    db::Db,
    models::{MessageStatus, MessageType, UserMessage},
};

#[async_trait]
pub trait MessageRepository: Send + Sync {
    async fn insert_message(
        &self,
        room_id: Uuid,
        room_name: String,
        author_id: Option<Uuid>,
        author_username: Option<String>,
        content: &str,
        message_type: MessageType,
    ) -> Result<UserMessage, sqlx::Error>;

    async fn get_message_by_id(&self, message_id: Uuid)
    -> Result<Option<UserMessage>, sqlx::Error>;

    async fn get_room_messages(
        &self,
        room_id: Uuid,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UserMessage>, sqlx::Error>;

    async fn update_message_content(
        &self,
        message_id: Uuid,
        new_content: &str,
    ) -> Result<Option<UserMessage>, sqlx::Error>;

    async fn delete_message(&self, message_id: Uuid) -> Result<Option<UserMessage>, sqlx::Error>;
}

#[async_trait]
impl MessageRepository for Db {
    #[instrument(skip(self))]
    async fn insert_message(
        &self,
        room_id: Uuid,
        room_name: String,
        author_id: Option<Uuid>,
        author_username: Option<String>,
        content: &str,
        message_type: MessageType,
    ) -> Result<UserMessage, sqlx::Error> {
        sqlx::query_as::<_, UserMessage>(
            r#"
            INSERT INTO user_messages (id, room_id, room_name, author_id, author_username, content, message_type, status, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(room_id)
        .bind(room_name)
        .bind(author_id)
        .bind(author_username)
        .bind(content)
        .bind(message_type)
        .bind(MessageStatus::Sent)
        .bind(Utc::now())
        .fetch_one(self.pool())
        .await
    }

    #[instrument(skip(self))]
    async fn get_message_by_id(
        &self,
        message_id: Uuid,
    ) -> Result<Option<UserMessage>, sqlx::Error> {
        sqlx::query_as::<_, UserMessage>(
            r#"
            SELECT *
            FROM user_messages
            WHERE id = $1
            "#,
        )
        .bind(message_id)
        .fetch_optional(self.pool())
        .await
    }

    #[instrument(skip(self))]
    async fn get_room_messages(
        &self,
        room_id: Uuid,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<UserMessage>, sqlx::Error> {
        let mut messages = sqlx::query_as::<_, UserMessage>(
            r#"
            SELECT
                m.id, m.room_id, m.room_name, m.author_id, m.author_username, m.content,
                m.message_type,
                m.status,
                m.created_at
            FROM user_messages m
            JOIN room_members rm ON m.room_id = rm.room_id
            WHERE m.room_id = $1
            AND rm.user_id = $2
            AND (rm.left_at IS NULL OR m.created_at <= rm.left_at)
            AND m.created_at >= rm.joined_at
            ORDER BY m.created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(room_id)
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(self.pool())
        .await?;

        messages.reverse();
        Ok(messages)
    }

    #[instrument(skip(self))]
    async fn update_message_content(
        &self,
        message_id: Uuid,
        new_content: &str,
    ) -> Result<Option<UserMessage>, sqlx::Error> {
        sqlx::query_as::<_, UserMessage>(
            r#"
            UPDATE user_messages
            SET content = $1, status = 'edited'
            WHERE id = $2 AND status != 'deleted'
            RETURNING *
            "#,
        )
        .bind(new_content)
        .bind(message_id)
        .fetch_optional(self.pool())
        .await
    }

    #[instrument(skip(self))]
    async fn delete_message(&self, message_id: Uuid) -> Result<Option<UserMessage>, sqlx::Error> {
        sqlx::query_as::<_, UserMessage>(
            r#"
            UPDATE user_messages
            SET status = 'deleted', content = ''
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(message_id)
        .fetch_optional(self.pool())
        .await
    }
}
