use sqlx::PgPool;
use uuid::Uuid;

use crate::db::accounts::{self, Account, NewAccount};
use crate::store::StoreError;
use crate::store::account_store::AccountStore;

pub struct PgAccountStore {
    pool: PgPool,
}

impl PgAccountStore {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl AccountStore for PgAccountStore {
    async fn find_by_provider(
        &self,
        provider: &str,
        provider_account_id: &str,
    ) -> Result<Option<Account>, StoreError> {
        Ok(accounts::find_by_provider(&self.pool, provider, provider_account_id).await?)
    }

    async fn create(&self, new_account: NewAccount) -> Result<Account, StoreError> {
        Ok(accounts::create(&self.pool, new_account).await?)
    }

    async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<Account>, StoreError> {
        Ok(accounts::list_for_user(&self.pool, user_id).await?)
    }
}
