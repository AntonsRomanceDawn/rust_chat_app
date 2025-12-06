use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tracing::instrument;
use uuid::Uuid;

use crate::database::{db::Db, models::RoomMember};

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

    async fn get_rooms_info_for_user(&self, user_id: Uuid) -> Result<Vec<RoomMember>, sqlx::Error>;

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
        sqlx::query_as!(
            RoomMember,
            r#"
            DELETE FROM room_members
            WHERE room_id = $1 AND user_id = $2
            RETURNING *
            "#,
            room_id,
            user_id
        )
        .fetch_optional(self.pool())
        .await
    }

    #[instrument(skip(self))]
    async fn get_members(&self, room_id: Uuid) -> Result<Vec<RoomMember>, sqlx::Error> {
        sqlx::query_as!(
            RoomMember,
            r#"
            SELECT *
            FROM room_members
            WHERE room_id = $1
            "#,
            room_id
        )
        .fetch_all(self.pool())
        .await
    }

    #[instrument(skip(self))]
    async fn is_member(&self, room_id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            SELECT *
            FROM room_members
            WHERE room_id = $1 AND user_id = $2
            "#,
            room_id,
            user_id
        )
        .fetch_optional(self.pool())
        .await?;

        Ok(result.is_some())
    }

    #[instrument(skip(self))]
    async fn is_admin(&self, room_id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            r#"
            SELECT * FROM rooms
            WHERE id = $1 AND admin_id = $2
            "#,
            room_id,
            user_id
        )
        .fetch_optional(self.pool())
        .await?;

        Ok(result.is_some())
    }

    #[instrument(skip(self))]
    async fn get_rooms_info_for_user(&self, user_id: Uuid) -> Result<Vec<RoomMember>, sqlx::Error> {
        sqlx::query_as!(
            RoomMember,
            r#"
            SELECT * FROM room_members
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_all(self.pool())
        .await
    }

    #[instrument(skip(self))]
    async fn increment_unread_count(
        &self,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            UPDATE room_members
            SET unread_count = unread_count + 1
            WHERE room_id = $1 AND user_id = $2
            "#,
            room_id,
            user_id,
        )
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
        sqlx::query!(
            r#"
            UPDATE room_members
            SET last_read_at = NOW()
            , unread_count = 0
            WHERE room_id = $1 AND user_id = $2
            "#,
            room_id,
            user_id,
        )
        .execute(self.pool())
        .await?;

        Ok(())
    }
}
