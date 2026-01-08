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
    async fn update_key_backup(&self, user_id: Uuid, backup: &str) -> Result<(), sqlx::Error>;
}

#[async_trait]
impl UserRepository for Db {
    #[instrument(skip(self))]
    async fn get_user_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(r#"SELECT * FROM users WHERE id = $1"#)
            .bind(id)
            .fetch_optional(self.pool())
            .await
    }

    #[instrument(skip(self))]
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(r#"SELECT * FROM users WHERE username = $1"#)
            .bind(username)
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
        sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, username, password_hash, role, created_at)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (username) DO NOTHING
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(username)
        .bind(password_hash)
        .bind(role)
        .bind(Utc::now())
        .fetch_optional(self.pool())
        .await
    }

    #[instrument(skip(self))]
    async fn delete_user(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(r#"DELETE FROM users WHERE id = $1 RETURNING *"#)
            .bind(id)
            .fetch_optional(self.pool())
            .await
    }

    #[instrument(skip(self))]
    async fn search_users(&self, query: &str) -> Result<Vec<User>, sqlx::Error> {
        let pattern = format!("%{}%", query);
        sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users
            WHERE username ILIKE $1
            LIMIT 20
            "#,
        )
        .bind(pattern)
        .fetch_all(self.pool())
        .await
    }

    #[instrument(skip(self, backup))]
    async fn update_key_backup(&self, user_id: Uuid, backup: &str) -> Result<(), sqlx::Error> {
        sqlx::query(r#"UPDATE users SET encrypted_key_backup = $1 WHERE id = $2"#)
            .bind(backup)
            .bind(user_id)
            .execute(self.pool())
            .await?;
        Ok(())
    }
}
