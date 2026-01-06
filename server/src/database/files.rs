use async_trait::async_trait;
use chrono::Utc;
use tracing::instrument;
use uuid::Uuid;

use crate::database::{db::Db, models::FileRecord};

#[async_trait]
pub trait FileRepository: Send + Sync {
    async fn insert_file(
        &self,
        encrypted_data: Vec<u8>,
        encrypted_metadata: Option<Vec<u8>>,
        size_in_bytes: i64,
        file_hash: String,
    ) -> Result<FileRecord, sqlx::Error>;

    async fn get_file(&self, file_id: Uuid) -> Result<Option<FileRecord>, sqlx::Error>;

    async fn delete_file(&self, file_id: Uuid) -> Result<Option<FileRecord>, sqlx::Error>;
}

#[async_trait]
impl FileRepository for Db {
    #[instrument(skip(self, encrypted_data, encrypted_metadata))]
    async fn insert_file(
        &self,
        encrypted_data: Vec<u8>,
        encrypted_metadata: Option<Vec<u8>>,
        size_in_bytes: i64,
        file_hash: String,
    ) -> Result<FileRecord, sqlx::Error> {
        sqlx::query_as::<_, FileRecord>(
            r#"
            INSERT INTO files (id, encrypted_data, encrypted_metadata, size_in_bytes, file_hash, uploaded_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(encrypted_data)
        .bind(encrypted_metadata)
        .bind(size_in_bytes)
        .bind(file_hash)
        .bind(Utc::now())
        .fetch_one(self.pool())
        .await
    }

    #[instrument(skip(self))]
    async fn get_file(&self, file_id: Uuid) -> Result<Option<FileRecord>, sqlx::Error> {
        sqlx::query_as::<_, FileRecord>(
            r#"
            SELECT * FROM files WHERE id = $1
            "#,
        )
        .bind(file_id)
        .fetch_optional(self.pool())
        .await
    }

    #[instrument(skip(self))]
    async fn delete_file(&self, file_id: Uuid) -> Result<Option<FileRecord>, sqlx::Error> {
        sqlx::query_as::<_, FileRecord>(
            r#"
            DELETE FROM files WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(file_id)
        .fetch_optional(self.pool())
        .await
    }
}
