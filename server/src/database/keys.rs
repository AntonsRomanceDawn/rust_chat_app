use crate::database::{
    db::Db,
    models::{IdentityKey, OneTimePreKey, SignedPreKey},
};
use async_trait::async_trait;
use tracing::instrument;
use uuid::Uuid;

#[async_trait]
pub trait KeyRepository: Send + Sync {
    async fn upsert_identity_key(
        &self,
        user_id: Uuid,
        identity_key: String,
        registration_id: i32,
    ) -> Result<IdentityKey, sqlx::Error>;

    async fn upsert_signed_prekey(
        &self,
        user_id: Uuid,
        key_id: i32,
        public_key: String,
        signature: String,
    ) -> Result<SignedPreKey, sqlx::Error>;

    async fn upload_one_time_prekeys(
        &self,
        user_id: Uuid,
        keys: Vec<(i32, String)>,
    ) -> Result<(), sqlx::Error>;

    async fn get_identity_key(&self, user_id: Uuid) -> Result<Option<IdentityKey>, sqlx::Error>;

    async fn get_signed_prekey(&self, user_id: Uuid) -> Result<Option<SignedPreKey>, sqlx::Error>;

    async fn consume_one_time_prekey(
        &self,
        user_id: Uuid,
    ) -> Result<Option<OneTimePreKey>, sqlx::Error>;

    async fn get_prekey_bundle_counts(&self, user_id: Uuid) -> Result<i64, sqlx::Error>;
}

#[async_trait]
impl KeyRepository for Db {
    #[instrument(skip(self, identity_key))]
    async fn upsert_identity_key(
        &self,
        user_id: Uuid,
        identity_key: String,
        registration_id: i32,
    ) -> Result<IdentityKey, sqlx::Error> {
        sqlx::query_as::<_, IdentityKey>(
            r#"
            INSERT INTO identity_keys (user_id, identity_key, registration_id)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id) DO UPDATE
            SET identity_key = EXCLUDED.identity_key,
                registration_id = EXCLUDED.registration_id,
                created_at = NOW()
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(identity_key)
        .bind(registration_id)
        .fetch_one(self.pool())
        .await
    }

    #[instrument(skip(self, public_key, signature))]
    async fn upsert_signed_prekey(
        &self,
        user_id: Uuid,
        key_id: i32,
        public_key: String,
        signature: String,
    ) -> Result<SignedPreKey, sqlx::Error> {
        sqlx::query_as::<_, SignedPreKey>(
            r#"
            INSERT INTO signed_prekeys (id, user_id, key_id, public_key, signature)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (user_id, key_id) DO UPDATE
            SET public_key = EXCLUDED.public_key,
                signature = EXCLUDED.signature,
                created_at = NOW()
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(key_id)
        .bind(public_key)
        .bind(signature)
        .fetch_one(self.pool())
        .await
    }

    #[instrument(skip(self, keys))]
    async fn upload_one_time_prekeys(
        &self,
        user_id: Uuid,
        keys: Vec<(i32, String)>,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool().begin().await?;
        // First, clear existing keys to prevent serving stale keys that the client has lost (e.g. after a reinstall/storage wipe)
        sqlx::query("DELETE FROM one_time_prekeys WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        let (key_ids, public_keys): (Vec<i32>, Vec<String>) = keys.into_iter().unzip();

        sqlx::query(
            r#"
            INSERT INTO one_time_prekeys (user_id, key_id, public_key)
            SELECT $1, * FROM UNNEST($2::int[], $3::text[])
            ON CONFLICT (user_id, key_id) DO NOTHING
            "#,
        )
        .bind(user_id)
        .bind(&key_ids)
        .bind(&public_keys)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_identity_key(&self, user_id: Uuid) -> Result<Option<IdentityKey>, sqlx::Error> {
        sqlx::query_as::<_, IdentityKey>("SELECT * FROM identity_keys WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(self.pool())
            .await
    }

    #[instrument(skip(self))]
    async fn get_signed_prekey(&self, user_id: Uuid) -> Result<Option<SignedPreKey>, sqlx::Error> {
        sqlx::query_as::<_, SignedPreKey>(
            "SELECT * FROM signed_prekeys WHERE user_id = $1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(self.pool())
        .await
    }

    #[instrument(skip(self))]
    async fn consume_one_time_prekey(
        &self,
        user_id: Uuid,
    ) -> Result<Option<OneTimePreKey>, sqlx::Error> {
        sqlx::query_as::<_, OneTimePreKey>(
            r#"
            DELETE FROM one_time_prekeys
            WHERE user_id = $1 AND key_id = (
                SELECT key_id FROM one_time_prekeys
                WHERE user_id = $1
                ORDER BY key_id ASC
                LIMIT 1
                FOR UPDATE SKIP LOCKED
            )
            RETURNING *
            "#,
        )
        .bind(user_id)
        .fetch_optional(self.pool())
        .await
    }

    #[instrument(skip(self))]
    async fn get_prekey_bundle_counts(&self, user_id: Uuid) -> Result<i64, sqlx::Error> {
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) as count FROM one_time_prekeys WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(self.pool())
                .await?;

        Ok(count.0)
    }
}
