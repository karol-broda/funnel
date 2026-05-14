use uuid::Uuid;

use super::StoreError;
use crate::db::accounts::{Account, NewAccount};

#[async_trait::async_trait]
pub trait AccountStore: Send + Sync {
    async fn find_by_provider(
        &self,
        provider: &str,
        provider_account_id: &str,
    ) -> Result<Option<Account>, StoreError>;
    async fn create(&self, new_account: NewAccount) -> Result<Account, StoreError>;
    async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<Account>, StoreError>;
}
