use async_trait::async_trait;
use chrono::Utc;
use tracing::instrument;
use uuid::Uuid;

use crate::database::{db::Db, models::UserMessage};

#[async_trait]
pub trait MessageRepository: Send + Sync {
    async fn insert_message(
        &self,
        room_id: Uuid,
        room_name: String,
        author_id: Uuid,
        author_username: String,
        content: &str,
        message_type: i32,
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
        author_id: Uuid,
        author_username: String,
        content: &str,
        message_type: i32,
    ) -> Result<UserMessage, sqlx::Error> {
        sqlx::query_as!(
            UserMessage,
            r#"
            INSERT INTO user_messages (id, room_id, room_name, author_id, author_username, content, message_type, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
            Uuid::new_v4(),
            room_id,
            room_name,
            author_id,
            author_username,
            content,
            message_type,
            Utc::now()
        )
        .fetch_one(self.pool())
        .await
    }

    #[instrument(skip(self))]
    async fn get_message_by_id(
        &self,
        message_id: Uuid,
    ) -> Result<Option<UserMessage>, sqlx::Error> {
        sqlx::query_as!(
            UserMessage,
            r#"
            SELECT * FROM user_messages
            WHERE id = $1
            "#,
            message_id
        )
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
        let mut messages = sqlx::query_as!(
            UserMessage,
            r#"
            SELECT m.* FROM user_messages m
            JOIN room_members rm ON m.room_id = rm.room_id
            WHERE m.room_id = $1
            AND rm.user_id = $2
            AND m.created_at >= rm.joined_at
            ORDER BY m.created_at DESC
            LIMIT $3 OFFSET $4
            "#,
            room_id,
            user_id,
            limit,
            offset
        )
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
        sqlx::query_as!(
            UserMessage,
            r#"
            UPDATE user_messages
            SET content = $1
            WHERE id = $2
            RETURNING *
            "#,
            new_content,
            message_id
        )
        .fetch_optional(self.pool())
        .await
    }

    #[instrument(skip(self))]
    async fn delete_message(&self, message_id: Uuid) -> Result<Option<UserMessage>, sqlx::Error> {
        sqlx::query_as!(
            UserMessage,
            r#"DELETE FROM user_messages WHERE id = $1 RETURNING *"#,
            message_id
        )
        .fetch_optional(self.pool())
        .await
    }
}
