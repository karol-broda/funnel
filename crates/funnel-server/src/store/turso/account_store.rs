use std::sync::Arc;

use chrono::Utc;
use turso::Database;
use uuid::Uuid;

use super::{format_dt, map_err, parse_dt, parse_uuid};
use crate::db::accounts::{Account, NewAccount};
use crate::store::account_store::AccountStore;
use crate::store::StoreError;

pub struct TursoAccountStore {
    db: Arc<Database>,
}

impl TursoAccountStore {
    pub const fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

fn row_to_account(row: &turso::Row) -> Result<Account, StoreError> {
    Ok(Account {
        id: parse_uuid(&row.get::<String>(0).map_err(|e| map_err(&e))?)?,
        user_id: parse_uuid(&row.get::<String>(1).map_err(|e| map_err(&e))?)?,
        provider: row.get::<String>(2).map_err(|e| map_err(&e))?,
        provider_account_id: row.get::<String>(3).map_err(|e| map_err(&e))?,
        metadata: serde_json::from_str(&row.get::<String>(4).map_err(|e| map_err(&e))?)
            .map_err(|e| StoreError::Other(format!("invalid json: {e}")))?,
        created_at: parse_dt(&row.get::<String>(5).map_err(|e| map_err(&e))?)?,
        updated_at: parse_dt(&row.get::<String>(6).map_err(|e| map_err(&e))?)?,
    })
}

#[async_trait::async_trait]
impl AccountStore for TursoAccountStore {
    async fn find_by_provider(
        &self,
        provider: &str,
        provider_account_id: &str,
    ) -> Result<Option<Account>, StoreError> {
        let conn = self.db.connect().map_err(|e| map_err(&e))?;
        let mut rows = conn
            .query(
                "SELECT id, user_id, provider, provider_account_id, metadata, created_at, updated_at FROM accounts WHERE provider = ? AND provider_account_id = ?",
                turso::params![provider.to_string(), provider_account_id.to_string()],
            )
            .await
            .map_err(|e| map_err(&e))?;
        match rows.next().await.map_err(|e| map_err(&e))? {
            Some(row) => Ok(Some(row_to_account(&row)?)),
            None => Ok(None),
        }
    }

    async fn create(&self, new_account: NewAccount) -> Result<Account, StoreError> {
        let conn = self.db.connect().map_err(|e| map_err(&e))?;
        let id = Uuid::now_v7();
        let now = Utc::now();
        let now_str = format_dt(now);
        let metadata_str = serde_json::to_string(&new_account.metadata)
            .map_err(|e| StoreError::Other(format!("invalid json: {e}")))?;

        conn.execute(
            "INSERT INTO accounts (id, user_id, provider, provider_account_id, metadata, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
            turso::params![
                id.to_string(),
                new_account.user_id.to_string(),
                new_account.provider.clone(),
                new_account.provider_account_id.clone(),
                metadata_str,
                now_str.clone(),
                now_str
            ],
        )
        .await
        .map_err(|e| map_err(&e))?;

        Ok(Account {
            id,
            user_id: new_account.user_id,
            provider: new_account.provider,
            provider_account_id: new_account.provider_account_id,
            metadata: new_account.metadata,
            created_at: now,
            updated_at: now,
        })
    }

    async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<Account>, StoreError> {
        let conn = self.db.connect().map_err(|e| map_err(&e))?;
        let mut rows = conn
            .query(
                "SELECT id, user_id, provider, provider_account_id, metadata, created_at, updated_at FROM accounts WHERE user_id = ? ORDER BY created_at DESC",
                turso::params![user_id.to_string()],
            )
            .await
            .map_err(|e| map_err(&e))?;
        let mut accounts = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| map_err(&e))? {
            accounts.push(row_to_account(&row)?);
        }
        Ok(accounts)
    }
}
