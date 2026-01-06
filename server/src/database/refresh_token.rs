use async_trait::async_trait;
use chrono::{Duration, Utc};
use tracing::instrument;
use uuid::Uuid;

use crate::database::{db::Db, models::RefreshToken};

#[async_trait]
pub trait RefreshTokenRepository: Send + Sync {
    async fn get_refresh_token_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<RefreshToken>, sqlx::Error>;
    async fn insert_refresh_token_by_hash(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_in: Duration,
    ) -> Result<RefreshToken, sqlx::Error>;

    async fn delete_refresh_token_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<RefreshToken>, sqlx::Error>;

    // async fn delete_tokens_for_user(
    //     &self,
    //     user_id: Uuid,
    // ) -> Result<Option<RefreshToken>, sqlx::Error>;
}

#[async_trait]
impl RefreshTokenRepository for Db {
    #[instrument(skip(self))]
    async fn get_refresh_token_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<RefreshToken>, sqlx::Error> {
        sqlx::query_as::<_, RefreshToken>(
            r#"
            SELECT *
            FROM refresh_tokens
            WHERE token_hash = $1
            "#,
        )
        .bind(token_hash)
        .fetch_optional(self.pool())
        .await
    }

    #[instrument(skip(self))]
    async fn insert_refresh_token_by_hash(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_in: Duration,
    ) -> Result<RefreshToken, sqlx::Error> {
        let created_at = Utc::now();
        let expires_at = created_at + expires_in;

        sqlx::query_as::<_, RefreshToken>(
            r#"
            INSERT INTO refresh_tokens (
                id, user_id, token_hash, expires_at, created_at
            )
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .bind(created_at)
        .fetch_one(self.pool())
        .await
    }
    #[instrument(skip(self))]
    async fn delete_refresh_token_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<RefreshToken>, sqlx::Error> {
        sqlx::query_as::<_, RefreshToken>(
            r#"
            DELETE FROM refresh_tokens
            WHERE token_hash = $1
            RETURNING *
            "#,
        )
        .bind(token_hash)
        .fetch_optional(self.pool())
        .await
    }

    // async fn delete_tokens_for_user(
    //     &self,
    //     user_id: Uuid,
    // ) -> Result<Option<RefreshToken>, sqlx::Error> {
    //     sqlx::query_as!(
    //         RefreshToken,
    //         r#"
    //         DELETE FROM refresh_tokens
    //         WHERE user_id = $1
    //         RETURNING *
    //         "#,
    //         user_id,
    //     )
    //     .fetch_optional(self.pool())
    //     .await
    // }
}
