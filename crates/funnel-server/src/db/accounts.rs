use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Account {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub provider_account_id: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct NewAccount {
    pub user_id: Uuid,
    pub provider: String,
    pub provider_account_id: String,
    pub metadata: serde_json::Value,
}

pub async fn find_by_provider(
    pool: &PgPool,
    provider: &str,
    provider_account_id: &str,
) -> Result<Option<Account>, sqlx::Error> {
    sqlx::query_as::<_, Account>(
        "SELECT * FROM accounts WHERE provider = $1 AND provider_account_id = $2",
    )
    .bind(provider)
    .bind(provider_account_id)
    .fetch_optional(pool)
    .await
}

pub async fn create(pool: &PgPool, new_account: NewAccount) -> Result<Account, sqlx::Error> {
    sqlx::query_as::<_, Account>(
        r"
        INSERT INTO accounts (user_id, provider, provider_account_id, metadata)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        ",
    )
    .bind(new_account.user_id)
    .bind(&new_account.provider)
    .bind(&new_account.provider_account_id)
    .bind(&new_account.metadata)
    .fetch_one(pool)
    .await
}

pub async fn list_for_user(pool: &PgPool, user_id: Uuid) -> Result<Vec<Account>, sqlx::Error> {
    sqlx::query_as::<_, Account>(
        "SELECT * FROM accounts WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}
