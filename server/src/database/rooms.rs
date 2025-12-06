use async_trait::async_trait;
use chrono::Utc;
use tracing::instrument;
use uuid::Uuid;

use crate::database::{db::Db, models::Room};

#[async_trait]
pub trait RoomRepository: Send + Sync {
    async fn create_room(
        &self,
        name: &str,
        creator_id: Uuid,
        creator_name: String,
    ) -> Result<Room, sqlx::Error>;

    async fn get_room_by_id(&self, room_id: Uuid) -> Result<Option<Room>, sqlx::Error>;

    async fn update_room_name(
        &self,
        room_id: Uuid,
        name: &str,
    ) -> Result<Option<Room>, sqlx::Error>;

    async fn delete_room(&self, room_id: Uuid) -> Result<Option<Room>, sqlx::Error>;

    async fn leave_room(&self, room_id: Uuid, user_id: Uuid) -> Result<Option<Room>, sqlx::Error>;

    // async fn get_user_rooms(&self, user_id: Uuid) -> Result<Vec<Room>, sqlx::Error>;
}

#[async_trait]
impl RoomRepository for Db {
    #[instrument(skip(self))]
    async fn create_room(
        &self,
        name: &str,
        creator_id: Uuid,
        creator_username: String,
    ) -> Result<Room, sqlx::Error> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let mut tx = self.pool().begin().await?;

        let room = sqlx::query_as!(
            Room,
            r#"
            INSERT INTO rooms (id, name, creator_id, creator_username, admin_id, admin_username, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
            id,
            name,
            creator_id,
            creator_username,
            creator_id,
            creator_username,
            now
        )
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO room_members (room_id, room_name, user_id, username, joined_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            id,
            name,
            creator_id,
            creator_username,
            now
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(room)
    }

    #[instrument(skip(self))]
    async fn get_room_by_id(&self, room_id: Uuid) -> Result<Option<Room>, sqlx::Error> {
        sqlx::query_as!(Room, r#"SELECT * FROM rooms WHERE id = $1"#, room_id)
            .fetch_optional(self.pool())
            .await
    }

    #[instrument(skip(self))]
    async fn update_room_name(
        &self,
        room_id: Uuid,
        name: &str,
    ) -> Result<Option<Room>, sqlx::Error> {
        sqlx::query_as!(
            Room,
            r#"UPDATE rooms SET name = $1 WHERE id = $2 RETURNING *"#,
            name,
            room_id
        )
        .fetch_optional(self.pool())
        .await
    }

    #[instrument(skip(self))]
    async fn delete_room(&self, room_id: Uuid) -> Result<Option<Room>, sqlx::Error> {
        sqlx::query_as!(
            Room,
            r#"DELETE FROM rooms WHERE id = $1 RETURNING *"#,
            room_id
        )
        .fetch_optional(self.pool())
        .await
    }

    #[instrument(skip(self))]
    async fn leave_room(&self, room_id: Uuid, user_id: Uuid) -> Result<Option<Room>, sqlx::Error> {
        let mut tx = self.pool().begin().await?;

        let room = sqlx::query_as!(Room, r#"SELECT * FROM rooms WHERE id = $1"#, room_id)
            .fetch_optional(&mut *tx)
            .await?;

        if room.is_none() {
            tx.commit().await?;
            return Ok(None);
        }

        let member_ids: Vec<Uuid> = sqlx::query!(
            r#"SELECT user_id FROM room_members WHERE room_id = $1"#,
            room_id
        )
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|m| m.user_id)
        .collect();

        if member_ids.len() == 1 {
            sqlx::query!(r#"DELETE FROM rooms WHERE id = $1"#, room_id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            return Ok(room);
        }

        sqlx::query!(
            r#"DELETE FROM room_members WHERE room_id = $1 AND user_id = $2"#,
            room_id,
            user_id
        )
        .execute(&mut *tx)
        .await?;

        if let Some(room) = &room {
            if room.admin_id == user_id {
                let new_admin = member_ids
                    .into_iter()
                    .find(|&id| id != user_id)
                    .ok_or(sqlx::Error::RowNotFound)?;

                sqlx::query!(
                    r#"UPDATE rooms SET admin_id = $1 WHERE id = $2"#,
                    new_admin,
                    room_id
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(room)
    }

    // #[instrument(skip(self))]
    // async fn get_user_rooms(&self, user_id: Uuid) -> Result<Vec<Room>, sqlx::Error> {
    //     sqlx::query_as!(
    //         Room,
    //         r#"
    //         SELECT r.*
    //         FROM rooms r
    //         JOIN room_members rm ON r.id = rm.room_id
    //         WHERE rm.user_id = $1
    //         ORDER BY r.created_at DESC
    //         "#,
    //         user_id
    //     )
    //     .fetch_all(self.pool())
    //     .await
    // }
}
