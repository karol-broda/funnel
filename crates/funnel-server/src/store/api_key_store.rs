use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::{BoxFuture, StoreError};
use crate::db::api_keys::{ApiKey, ApiKeyView};

pub trait ApiKeyStore: Send + Sync {
    fn create(
        &self,
        user_id: Uuid,
        name: &str,
        scopes: &serde_json::Value,
        expires_at: Option<DateTime<Utc>>,
    ) -> BoxFuture<'_, Result<(String, ApiKeyView), StoreError>>;
    fn validate(&self, plaintext: &str) -> BoxFuture<'_, Result<Option<ApiKey>, StoreError>>;
    fn list_for_user(&self, user_id: Uuid) -> BoxFuture<'_, Result<Vec<ApiKeyView>, StoreError>>;
    fn revoke(&self, key_id: Uuid, user_id: Uuid) -> BoxFuture<'_, Result<bool, StoreError>>;
    fn revoke_by_name(&self, user_id: Uuid, name: &str) -> BoxFuture<'_, Result<bool, StoreError>>;
}
