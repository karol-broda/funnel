use sqlx::PgPool;
use uuid::Uuid;

use crate::db::api_keys::{self, ApiKey, ApiKeyView};
use crate::store::api_key_store::ApiKeyStore;
use crate::store::{BoxFuture, StoreError};

pub struct PgApiKeyStore {
    pool: PgPool,
}

impl PgApiKeyStore {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl ApiKeyStore for PgApiKeyStore {
    fn create(
        &self,
        user_id: Uuid,
        name: &str,
        scopes: &serde_json::Value,
    ) -> BoxFuture<'_, Result<(String, ApiKeyView), StoreError>> {
        let name = name.to_string();
        let scopes = scopes.clone();
        Box::pin(async move {
            let (plaintext, key) = api_keys::create(&self.pool, user_id, &name, &scopes).await?;
            Ok((plaintext, key.into()))
        })
    }

    fn validate(&self, plaintext: &str) -> BoxFuture<'_, Result<Option<ApiKey>, StoreError>> {
        let plaintext = plaintext.to_string();
        Box::pin(async move { Ok(api_keys::validate(&self.pool, &plaintext).await?) })
    }

    fn list_for_user(&self, user_id: Uuid) -> BoxFuture<'_, Result<Vec<ApiKeyView>, StoreError>> {
        Box::pin(async move {
            let keys = api_keys::list_for_user(&self.pool, user_id).await?;
            Ok(keys.into_iter().map(Into::into).collect())
        })
    }

    fn revoke(&self, key_id: Uuid, user_id: Uuid) -> BoxFuture<'_, Result<bool, StoreError>> {
        Box::pin(async move { Ok(api_keys::revoke(&self.pool, key_id, user_id).await?) })
    }
}
