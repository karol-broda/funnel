use uuid::Uuid;

use crate::db::api_keys::{ApiKey, ApiKeyView};
use super::{BoxFuture, StoreError};

#[allow(dead_code)]
pub trait ApiKeyStore: Send + Sync {
    fn create(&self, user_id: Uuid, name: &str) -> BoxFuture<'_, Result<(String, ApiKeyView), StoreError>>;
    fn validate(&self, plaintext: &str) -> BoxFuture<'_, Result<Option<ApiKey>, StoreError>>;
    fn list_for_user(&self, user_id: Uuid) -> BoxFuture<'_, Result<Vec<ApiKeyView>, StoreError>>;
    fn revoke(&self, key_id: Uuid, user_id: Uuid) -> BoxFuture<'_, Result<bool, StoreError>>;
}
