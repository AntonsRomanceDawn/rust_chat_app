use async_trait::async_trait;
use chrono::Utc;
use tracing::instrument;
use uuid::Uuid;

use crate::database::{
    db::Db,
    models::{User, UserRole},
};

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn get_user_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error>;
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, sqlx::Error>;
    async fn insert_user(
        &self,
        username: &str,
        password_hash: &str,
        role: UserRole,
    ) -> Result<Option<User>, sqlx::Error>;
    async fn delete_user(&self, id: Uuid) -> Result<Option<User>, sqlx::Error>;
    async fn search_users(&self, query: &str) -> Result<Vec<User>, sqlx::Error>;
}

#[async_trait]
impl UserRepository for Db {
    #[instrument(skip(self))]
    async fn get_user_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(User, r#"SELECT * FROM users WHERE id = $1"#, id)
            .fetch_optional(self.pool())
            .await
    }

    #[instrument(skip(self))]
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(User, r#"SELECT * FROM users WHERE username = $1"#, username)
            .fetch_optional(self.pool())
            .await
    }

    #[instrument(skip(self, password_hash))]
    async fn insert_user(
        &self,
        username: &str,
        password_hash: &str,
        role: UserRole,
    ) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (id, username, password_hash, role, created_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (username) DO NOTHING
            RETURNING *
            "#,
            Uuid::new_v4(),
            username,
            password_hash,
            role.to_string(),
            Utc::now(),
        )
        .fetch_optional(self.pool())
        .await
    }

    #[instrument(skip(self))]
    async fn delete_user(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as!(User, r#"DELETE FROM users WHERE id = $1 RETURNING *"#, id)
            .fetch_optional(self.pool())
            .await
    }

    #[instrument(skip(self))]
    async fn search_users(&self, query: &str) -> Result<Vec<User>, sqlx::Error> {
        let pattern = format!("%{}%", query);
        sqlx::query_as!(
            User,
            r#"
            SELECT * FROM users
            WHERE username ILIKE $1
            LIMIT 20
            "#,
            pattern
        )
        .fetch_all(self.pool())
        .await
    }
}
