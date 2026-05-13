use sqlx::PgPool;
use uuid::Uuid;

use crate::db::accounts::{self, Account, NewAccount};
use crate::store::account_store::AccountStore;
use crate::store::{BoxFuture, StoreError};

pub struct PgAccountStore {
    pool: PgPool,
}

impl PgAccountStore {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl AccountStore for PgAccountStore {
    fn find_by_provider(
        &self,
        provider: &str,
        provider_account_id: &str,
    ) -> BoxFuture<'_, Result<Option<Account>, StoreError>> {
        let provider = provider.to_string();
        let provider_account_id = provider_account_id.to_string();
        Box::pin(async move {
            Ok(accounts::find_by_provider(&self.pool, &provider, &provider_account_id).await?)
        })
    }

    fn create(&self, new_account: NewAccount) -> BoxFuture<'_, Result<Account, StoreError>> {
        Box::pin(async move { Ok(accounts::create(&self.pool, new_account).await?) })
    }

    fn list_for_user(&self, user_id: Uuid) -> BoxFuture<'_, Result<Vec<Account>, StoreError>> {
        Box::pin(async move { Ok(accounts::list_for_user(&self.pool, user_id).await?) })
    }
}
