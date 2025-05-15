use chrono::{DateTime, Utc};
use funnel_core::api::ApiScope;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::api_keys::{self, ApiKey, ApiKeyView};
use crate::store::StoreError;
use crate::store::api_key_store::ApiKeyStore;

pub struct PgApiKeyStore {
    pool: PgPool,
}

impl PgApiKeyStore {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ApiKeyStore for PgApiKeyStore {
    async fn create(
        &self,
        user_id: Uuid,
        name: &str,
        scopes: &[ApiScope],
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<(String, ApiKeyView), StoreError> {
        let (plaintext, key) =
            api_keys::create(&self.pool, user_id, name, scopes, expires_at).await?;
        Ok((plaintext, key.into()))
    }

    async fn validate(&self, plaintext: &str) -> Result<Option<ApiKey>, StoreError> {
        Ok(api_keys::validate(&self.pool, plaintext).await?)
    }

    async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<ApiKeyView>, StoreError> {
        let keys = api_keys::list_for_user(&self.pool, user_id).await?;
        Ok(keys.into_iter().map(Into::into).collect())
    }

    async fn revoke(&self, key_id: Uuid, user_id: Uuid) -> Result<bool, StoreError> {
        Ok(api_keys::revoke(&self.pool, key_id, user_id).await?)
    }

    async fn revoke_by_name(&self, user_id: Uuid, name: &str) -> Result<bool, StoreError> {
        Ok(api_keys::revoke_by_name(&self.pool, user_id, name).await?)
    }
}
