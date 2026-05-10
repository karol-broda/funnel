use std::sync::{PoisonError, RwLock};

use chrono::Utc;
use uuid::Uuid;

use crate::db::accounts::{Account, NewAccount};
use crate::store::account_store::AccountStore;
use crate::store::{BoxFuture, StoreError};

pub struct InMemoryAccountStore {
    accounts: RwLock<Vec<Account>>,
}

impl InMemoryAccountStore {
    pub const fn new() -> Self {
        Self {
            accounts: RwLock::new(Vec::new()),
        }
    }
}

impl AccountStore for InMemoryAccountStore {
    fn find_by_provider(
        &self,
        provider: &str,
        provider_account_id: &str,
    ) -> BoxFuture<'_, Result<Option<Account>, StoreError>> {
        let provider = provider.to_string();
        let provider_account_id = provider_account_id.to_string();
        Box::pin(async move {
            let accounts = self.accounts.read().unwrap_or_else(PoisonError::into_inner);
            Ok(accounts
                .iter()
                .find(|a| a.provider == provider && a.provider_account_id == provider_account_id)
                .cloned())
        })
    }

    fn create(&self, new_account: NewAccount) -> BoxFuture<'_, Result<Account, StoreError>> {
        Box::pin(async move {
            let now = Utc::now();
            let account = Account {
                id: Uuid::now_v7(),
                user_id: new_account.user_id,
                provider: new_account.provider,
                provider_account_id: new_account.provider_account_id,
                metadata: new_account.metadata,
                created_at: now,
                updated_at: now,
            };
            {
                let mut accounts = self
                    .accounts
                    .write()
                    .unwrap_or_else(PoisonError::into_inner);

                if accounts.iter().any(|a| {
                    a.provider == account.provider
                        && a.provider_account_id == account.provider_account_id
                }) {
                    return Err(StoreError::Conflict(
                        "account already exists for this provider".into(),
                    ));
                }

                accounts.push(account.clone());
            }
            Ok(account)
        })
    }

    fn list_for_user(&self, user_id: Uuid) -> BoxFuture<'_, Result<Vec<Account>, StoreError>> {
        Box::pin(async move {
            let accounts = self.accounts.read().unwrap_or_else(PoisonError::into_inner);
            Ok(accounts
                .iter()
                .filter(|a| a.user_id == user_id)
                .cloned()
                .collect())
        })
    }
}
