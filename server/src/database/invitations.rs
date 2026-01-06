use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tracing::instrument;
use uuid::Uuid;

use crate::database::{
    db::Db,
    models::{Invitation, InvitationStatus},
};

#[async_trait]
pub trait InvitationRepository: Send + Sync {
    async fn create_invitation(
        &self,
        room_id: Uuid,
        room_name: String,
        invitee_id: Uuid,
        invitee_username: String,
        inviter_id: Uuid,
        inviter_username: String,
    ) -> Result<Option<Invitation>, sqlx::Error>;

    async fn update_invitation_status(
        &self,
        invitation_id: Uuid,
        status: InvitationStatus,
    ) -> Result<Option<Invitation>, sqlx::Error>;

    // async fn delete_invitation(
    //     &self,
    //     invitation_id: Uuid,
    // ) -> Result<Option<Invitation>, sqlx::Error>;

    async fn get_invitation_by_id(
        &self,
        invitation_id: Uuid,
    ) -> Result<Option<Invitation>, sqlx::Error>;

    async fn get_pending_invitations_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<Invitation>, sqlx::Error>;

    async fn consume_invitations_and_join_room(
        &self,
        room_id: Uuid,
        room_name: String,
        user_id: Uuid,
        username: String,
        joined_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error>;
}

#[async_trait]
impl InvitationRepository for Db {
    #[instrument(skip(self))]
    async fn create_invitation(
        &self,
        room_id: Uuid,
        room_name: String,
        invitee_id: Uuid,
        invitee_username: String,
        inviter_id: Uuid,
        inviter_username: String,
    ) -> Result<Option<Invitation>, sqlx::Error> {
        sqlx::query_as::<_, Invitation>(
            r#"
            INSERT INTO invitations (id, room_id, room_name, invitee_id, invitee_username, inviter_id, inviter_username, status, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (room_id, invitee_id, inviter_id) WHERE status = 'pending'
            DO NOTHING
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(room_id)
        .bind(room_name)
        .bind(invitee_id)
        .bind(invitee_username)
        .bind(inviter_id)
        .bind(inviter_username)
        .bind(InvitationStatus::Pending.to_string())
        .bind(Utc::now())
        .fetch_optional(self.pool())
        .await
    }

    #[instrument(skip(self))]
    async fn update_invitation_status(
        &self,
        invitation_id: Uuid,
        status: InvitationStatus,
    ) -> Result<Option<Invitation>, sqlx::Error> {
        sqlx::query_as::<_, Invitation>(
            r#"
            UPDATE invitations
            SET status = $1
            WHERE id = $2
            RETURNING *
            "#,
        )
        .bind(status.to_string())
        .bind(invitation_id)
        .fetch_optional(self.pool())
        .await
    }

    // async fn delete_invitation(
    //     &self,
    //     invitation_id: Uuid,
    // ) -> Result<Option<Invitation>, sqlx::Error> {
    //     sqlx::query_as!(
    //         Invitation,
    //         r#"
    //         DELETE FROM invitations
    //         WHERE id = $1
    //         RETURNING *
    //         "#,
    //         invitation_id
    //     )
    //     .fetch_optional(self.pool())
    //     .await
    // }
    #[instrument(skip(self))]
    async fn get_invitation_by_id(
        &self,
        invitation_id: Uuid,
    ) -> Result<Option<Invitation>, sqlx::Error> {
        sqlx::query_as::<_, Invitation>(
            r#"
            SELECT * FROM invitations WHERE id = $1
            "#,
        )
        .bind(invitation_id)
        .fetch_optional(self.pool())
        .await
    }
    #[instrument(skip(self))]
    async fn get_pending_invitations_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<Invitation>, sqlx::Error> {
        sqlx::query_as::<_, Invitation>(
            r#"
            SELECT * FROM invitations
            WHERE invitee_id = $1 AND status = $2
            "#,
        )
        .bind(user_id)
        .bind(InvitationStatus::Pending.to_string())
        .fetch_all(self.pool())
        .await
    }
    #[instrument(skip(self))]
    async fn consume_invitations_and_join_room(
        &self,
        room_id: Uuid,
        room_name: String,
        user_id: Uuid,
        username: String,
        joined_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool().begin().await?;

        sqlx::query(
            r#"
            UPDATE invitations
            SET status = $3
            WHERE room_id = $1 AND invitee_id = $2 AND status = 'pending'
            "#,
        )
        .bind(room_id)
        .bind(user_id)
        .bind(InvitationStatus::Accepted.to_string())
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO room_members (room_id, room_name, user_id, username, joined_at, last_read_at, unread_count)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (room_id, user_id) DO NOTHING
            "#,
        )
        .bind(room_id)
        .bind(room_name)
        .bind(user_id)
        .bind(username)
        .bind(joined_at)
        .bind(Utc::now())
        .bind(0)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }
}
