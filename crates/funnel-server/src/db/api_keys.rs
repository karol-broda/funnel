use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use funnel_core::api::ApiScope;
use funnel_core::auth::token as auth;

pub use funnel_core::api::ApiKeyView;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub key_hash: String,
    pub key_prefix: String,
    pub scopes: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl ApiKey {
    pub fn has_scope(&self, scope: ApiScope) -> bool {
        self.parsed_scopes().contains(&scope)
    }

    pub fn parsed_scopes(&self) -> Vec<ApiScope> {
        self.scopes
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .filter_map(|s| {
                        serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl From<ApiKey> for ApiKeyView {
    fn from(key: ApiKey) -> Self {
        let scopes = key.parsed_scopes();
        Self {
            id: key.id,
            name: key.name,
            key_prefix: key.key_prefix,
            scopes,
            created_at: key.created_at,
            revoked_at: key.revoked_at,
            expires_at: key.expires_at,
        }
    }
}

pub fn default_scopes() -> Vec<ApiScope> {
    vec![ApiScope::Management, ApiScope::Tunnels]
}

fn scopes_to_json(scopes: &[ApiScope]) -> serde_json::Value {
    serde_json::Value::Array(
        scopes
            .iter()
            .map(|s| serde_json::to_value(s).unwrap_or_default())
            .collect(),
    )
}

pub async fn create(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
    scopes: &[ApiScope],
    expires_at: Option<DateTime<Utc>>,
) -> Result<(String, ApiKey), sqlx::Error> {
    let plaintext = auth::generate_api_key()
        .map_err(|e| sqlx::Error::Protocol(format!("failed to generate api key: {e}")))?;
    let hash = auth::hash_token(&plaintext);
    let prefix = auth::ApiKeyPrefix::from_key(&plaintext);
    let scopes_json = scopes_to_json(scopes);

    let key = sqlx::query_as::<_, ApiKey>(
        r"
        INSERT INTO api_keys (user_id, name, key_hash, key_prefix, scopes, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        ",
    )
    .bind(user_id)
    .bind(name)
    .bind(&hash)
    .bind(prefix.as_ref())
    .bind(&scopes_json)
    .bind(expires_at)
    .fetch_one(pool)
    .await?;

    Ok((plaintext, key))
}

pub async fn validate(pool: &PgPool, plaintext: &str) -> Result<Option<ApiKey>, sqlx::Error> {
    let hash = auth::hash_token(plaintext);

    sqlx::query_as::<_, ApiKey>(
        "SELECT * FROM api_keys WHERE key_hash = $1 AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at > now())",
    )
    .bind(&hash)
    .fetch_optional(pool)
    .await
}

pub async fn list_for_user(pool: &PgPool, user_id: Uuid) -> Result<Vec<ApiKey>, sqlx::Error> {
    sqlx::query_as::<_, ApiKey>(
        "SELECT * FROM api_keys WHERE user_id = $1 AND revoked_at IS NULL ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn revoke(pool: &PgPool, key_id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE api_keys SET revoked_at = now() WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL",
    )
    .bind(key_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn revoke_by_name(pool: &PgPool, user_id: Uuid, name: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE api_keys SET revoked_at = now() WHERE user_id = $1 AND name = $2 AND revoked_at IS NULL",
    )
    .bind(user_id)
    .bind(name)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}
