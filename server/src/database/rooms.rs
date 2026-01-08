use async_trait::async_trait;
use chrono::Utc;
use sqlx::Row;
use tracing::instrument;
use uuid::Uuid;

use crate::database::{
    db::Db,
    models::{Invitation, Room},
};

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

    async fn leave_room(
        &self,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<(Vec<Invitation>, Room)>, sqlx::Error>;

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

        let room = sqlx::query_as::<_, Room>(
            r#"
            INSERT INTO rooms (id, name, creator_id, creator_username, admin_id, admin_username, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(creator_id)
        .bind(&creator_username)
        .bind(creator_id)
        .bind(&creator_username)
        .bind(now)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO room_members (id, room_id, room_name, user_id, username, joined_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(id)
        .bind(name)
        .bind(creator_id)
        .bind(&creator_username)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(room)
    }

    #[instrument(skip(self))]
    async fn get_room_by_id(&self, room_id: Uuid) -> Result<Option<Room>, sqlx::Error> {
        sqlx::query_as::<_, Room>(r#"SELECT * FROM rooms WHERE id = $1"#)
            .bind(room_id)
            .fetch_optional(self.pool())
            .await
    }

    #[instrument(skip(self))]
    async fn update_room_name(
        &self,
        room_id: Uuid,
        name: &str,
    ) -> Result<Option<Room>, sqlx::Error> {
        sqlx::query_as::<_, Room>(r#"UPDATE rooms SET name = $1 WHERE id = $2 RETURNING *"#)
            .bind(name)
            .bind(room_id)
            .fetch_optional(self.pool())
            .await
    }

    #[instrument(skip(self))]
    async fn delete_room(&self, room_id: Uuid) -> Result<Option<Room>, sqlx::Error> {
        sqlx::query_as::<_, Room>(r#"DELETE FROM rooms WHERE id = $1 RETURNING *"#)
            .bind(room_id)
            .fetch_optional(self.pool())
            .await
    }

    #[instrument(skip(self))]
    async fn leave_room(
        &self,
        room_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<(Vec<Invitation>, Room)>, sqlx::Error> {
        // pending invitations for the room
        let mut pending_invs = Vec::<Invitation>::new();

        let mut tx = self.pool().begin().await?;

        let room = match sqlx::query_as::<_, Room>(r#"SELECT * FROM rooms WHERE id = $1"#)
            .bind(room_id)
            .fetch_optional(&mut *tx)
            .await?
        {
            Some(room) => room,
            None => {
                tx.commit().await?;
                return Ok(None);
            }
        };

        // Check if user is currently an active member (not already left/kicked)
        let is_active_member = sqlx::query(
            r#"SELECT 1 FROM room_members WHERE room_id = $1 AND user_id = $2 AND left_at IS NULL"#,
        )
        .bind(room_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?
        .is_some();

        if !is_active_member {
            // User is not active (already left or kicked).
            // If they are calling leave_room, it means they want to hide it from their list.
            sqlx::query(
                r#"UPDATE room_members SET is_visible = FALSE WHERE room_id = $1 AND user_id = $2"#,
            )
            .bind(room_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

            tx.commit().await?;
            return Ok(Some((pending_invs, room)));
        }

        // User IS active. They are leaving now.

        let member_ids: Vec<Uuid> = sqlx::query(
            r#"SELECT user_id FROM room_members WHERE room_id = $1 AND left_at IS NULL"#,
        )
        .bind(room_id)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|m| m.get::<Uuid, _>("user_id"))
        .collect();

        if member_ids.len() == 1 {
            // Last member leaving. First fetch Invitations to notify users.
            pending_invs =
                sqlx::query_as::<_, Invitation>(r#"SELECT * FROM invitations WHERE room_id = $1"#)
                    .bind(room_id)
                    .fetch_all(&mut *tx)
                    .await?;

            sqlx::query(r#"DELETE FROM rooms WHERE id = $1"#)
                .bind(room_id)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            return Ok(Some((pending_invs, room)));
        }

        // Standard leave: set left_at and hide it.
        sqlx::query(r#"UPDATE room_members SET left_at = $1, is_visible = FALSE WHERE room_id = $2 AND user_id = $3 AND left_at IS NULL"#)
            .bind(Utc::now())
            .bind(room_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        if room.admin_id == user_id {
            let new_admin = member_ids
                .into_iter()
                .find(|&id| id != user_id)
                .ok_or(sqlx::Error::RowNotFound)?;

            sqlx::query(r#"UPDATE rooms SET admin_id = $1 WHERE id = $2"#)
                .bind(new_admin)
                .bind(room_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(Some((pending_invs, room)))
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
