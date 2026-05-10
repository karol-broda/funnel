use uuid::Uuid;

use super::{BoxFuture, StoreError};
use crate::db::accounts::{Account, NewAccount};

pub trait AccountStore: Send + Sync {
    fn find_by_provider(
        &self,
        provider: &str,
        provider_account_id: &str,
    ) -> BoxFuture<'_, Result<Option<Account>, StoreError>>;
    fn create(&self, new_account: NewAccount) -> BoxFuture<'_, Result<Account, StoreError>>;
    fn list_for_user(&self, user_id: Uuid) -> BoxFuture<'_, Result<Vec<Account>, StoreError>>;
}
