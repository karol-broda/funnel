use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::StoreError;
use crate::db::api_keys::{ApiKey, ApiKeyView};
use funnel_core::api::ApiScope;

#[async_trait::async_trait]
pub trait ApiKeyStore: Send + Sync {
    async fn create(
        &self,
        user_id: Uuid,
        name: &str,
        scopes: &[ApiScope],
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<(String, ApiKeyView), StoreError>;
    async fn validate(&self, plaintext: &str) -> Result<Option<ApiKey>, StoreError>;
    async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<ApiKeyView>, StoreError>;
    async fn revoke(&self, key_id: Uuid, user_id: Uuid) -> Result<bool, StoreError>;
    async fn revoke_by_name(&self, user_id: Uuid, name: &str) -> Result<bool, StoreError>;
}
