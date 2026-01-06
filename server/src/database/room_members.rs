use async_trait::async_trait;
use tracing::instrument;
use uuid::Uuid;

use crate::database::{
    db::Db,
    models::{RoomMember, UserMessage},
};
use sqlx::{FromRow, Row};

#[async_trait]
pub trait RoomMemberRepository: Send + Sync {
    // async fn add_member(
    //     &self,
    //     room_id: Uuid,
    //     room_name: String,
    //     user_id: Uuid,
    //     username: String,
    // ) -> Result<RoomMember, sqlx::Error>;

    async fn remove_member(
        &self,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<RoomMember>, sqlx::Error>;

    async fn get_members(&self, room_id: Uuid) -> Result<Vec<RoomMember>, sqlx::Error>;

    async fn is_member(&self, room_id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error>;

    async fn is_admin(&self, room_id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error>;

    async fn get_rooms_info_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(RoomMember, Option<UserMessage>)>, sqlx::Error>;

    async fn increment_unread_count(&self, room_id: Uuid, user_id: Uuid)
    -> Result<(), sqlx::Error>;

    async fn reset_last_read_and_count(
        &self,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), sqlx::Error>;
}

#[async_trait]
impl RoomMemberRepository for Db {
    // async fn add_member(
    //     &self,
    //     room_id: Uuid,
    //     room_name: String,
    //     user_id: Uuid,
    //     username: String,
    // ) -> Result<RoomMember, sqlx::Error> {
    //     sqlx::query_as!(
    //         RoomMember,
    //         r#"
    //         INSERT INTO room_members (room_id, room_name, user_id, username, joined_at)
    //         VALUES ($1, $2, $3, $4, $5)
    //         RETURNING *
    //         "#,
    //         room_id,
    //         room_name,
    //         user_id,
    //         username,
    //         Utc::now(),
    //     )
    //     .fetch_one(self.pool())
    //     .await
    // }

    #[instrument(skip(self))]
    async fn remove_member(
        &self,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<RoomMember>, sqlx::Error> {
        sqlx::query_as::<_, RoomMember>(
            r#"
            DELETE FROM room_members
            WHERE room_id = $1 AND user_id = $2
            RETURNING *
            "#,
        )
        .bind(room_id)
        .bind(user_id)
        .fetch_optional(self.pool())
        .await
    }

    #[instrument(skip(self))]
    async fn get_members(&self, room_id: Uuid) -> Result<Vec<RoomMember>, sqlx::Error> {
        sqlx::query_as::<_, RoomMember>(
            r#"
            SELECT *
            FROM room_members
            WHERE room_id = $1
            "#,
        )
        .bind(room_id)
        .fetch_all(self.pool())
        .await
    }

    #[instrument(skip(self))]
    async fn is_member(&self, room_id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            SELECT *
            FROM room_members
            WHERE room_id = $1 AND user_id = $2
            "#,
        )
        .bind(room_id)
        .bind(user_id)
        .fetch_optional(self.pool())
        .await?;

        Ok(result.is_some())
    }

    #[instrument(skip(self))]
    async fn is_admin(&self, room_id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            SELECT * FROM rooms
            WHERE id = $1 AND admin_id = $2
            "#,
        )
        .bind(room_id)
        .bind(user_id)
        .fetch_optional(self.pool())
        .await?;

        Ok(result.is_some())
    }

    #[instrument(skip(self))]
    async fn get_rooms_info_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(RoomMember, Option<UserMessage>)>, sqlx::Error> {
        sqlx::query(
            r#"
            SELECT
                rm.room_id, rm.room_name, rm.user_id, rm.username, rm.joined_at, rm.last_read_at, rm.unread_count,
                 msg.id, msg.room_id as msg_room_id, msg.room_name as msg_room_name,
                 msg.author_id, msg.author_username, msg.content, msg.message_type,
                 msg.status as msg_status,
                 msg.created_at
            FROM room_members rm
            LEFT JOIN LATERAL (
                SELECT * FROM user_messages msg
                WHERE msg.room_id = rm.room_id
                ORDER BY created_at DESC
                LIMIT 1
            ) msg ON true
            WHERE rm.user_id = $1
            ORDER BY COALESCE(msg.created_at, rm.joined_at) DESC
            "#,
        )
        .bind(user_id)
        .try_map(|row| {
            let member = RoomMember::from_row(&row)?;
            let msg_id = row.try_get::<Option<Uuid>, _>("id").ok().flatten();

            let last_message = match msg_id {
                Some(_) => {
                    // Manually construct UserMessage to handle potential ambiguous columns
                    // or nulls from the LEFT JOIN being picked up by from_row
                    Some(UserMessage {
                        id: row.try_get("id")?,
                        room_id: row.try_get("msg_room_id")?,
                        room_name: row.try_get("msg_room_name")?,
                        author_id: row.try_get("author_id")?,
                        author_username: row.try_get("author_username")?,
                        content: row.try_get("content")?,
                        message_type: row.try_get("message_type")?,
                        status: row.try_get("msg_status")?,
                        created_at: row.try_get("created_at")?,
                    })
                }
                None => None,
            };

            Ok((member, last_message))
        })
        .fetch_all(self.pool())
        .await
    }

    #[instrument(skip(self))]
    async fn increment_unread_count(
        &self,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE room_members
            SET unread_count = unread_count + 1
            WHERE room_id = $1 AND user_id = $2
            "#,
        )
        .bind(room_id)
        .bind(user_id)
        .execute(self.pool())
        .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn reset_last_read_and_count(
        &self,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE room_members
            SET last_read_at = NOW()
            , unread_count = 0
            WHERE room_id = $1 AND user_id = $2
            "#,
        )
        .bind(room_id)
        .bind(user_id)
        .execute(self.pool())
        .await?;

        Ok(())
    }
}
