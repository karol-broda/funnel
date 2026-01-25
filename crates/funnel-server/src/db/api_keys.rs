use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use funnel_core::auth;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub key_hash: String,
    pub key_prefix: String,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// A view of an API key safe for returning in API responses (no hash).
#[derive(Debug, Clone, Serialize)]
pub struct ApiKeyView {
    pub id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl From<ApiKey> for ApiKeyView {
    fn from(key: ApiKey) -> Self {
        Self {
            id: key.id,
            name: key.name,
            key_prefix: key.key_prefix,
            created_at: key.created_at,
            revoked_at: key.revoked_at,
        }
    }
}

/// Create a new API key for a user. Returns the full plaintext key (show once) and the DB record.
pub async fn create(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
) -> Result<(String, ApiKey), sqlx::Error> {
    let plaintext = auth::generate_api_key();
    let hash = auth::hash_token(&plaintext);
    let prefix = auth::ApiKeyPrefix::from_key(&plaintext);

    let key = sqlx::query_as::<_, ApiKey>(
        r#"
        INSERT INTO api_keys (user_id, name, key_hash, key_prefix)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(name)
    .bind(&hash)
    .bind(prefix.as_ref())
    .fetch_one(pool)
    .await?;

    Ok((plaintext, key))
}

/// Find the user ID associated with a plaintext API key, if it's valid and not revoked.
pub async fn validate(pool: &PgPool, plaintext: &str) -> Result<Option<ApiKey>, sqlx::Error> {
    let hash = auth::hash_token(plaintext);

    sqlx::query_as::<_, ApiKey>(
        "SELECT * FROM api_keys WHERE key_hash = $1 AND revoked_at IS NULL",
    )
    .bind(&hash)
    .fetch_optional(pool)
    .await
}

/// List all active (non-revoked) API keys for a user.
pub async fn list_for_user(pool: &PgPool, user_id: Uuid) -> Result<Vec<ApiKey>, sqlx::Error> {
    sqlx::query_as::<_, ApiKey>(
        "SELECT * FROM api_keys WHERE user_id = $1 AND revoked_at IS NULL ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

/// Revoke an API key by setting its `revoked_at` timestamp.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::users;

    async fn setup_user(pool: &PgPool) -> users::User {
        users::create(
            pool,
            users::NewUser {
                email: format!("apikey-test-{}@example.com", Uuid::now_v7()),
                name: Some("API Key Test".into()),
                avatar_url: None,
                provider: "github".into(),
                provider_id: Uuid::now_v7().to_string(),
            },
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    #[ignore = "requires database"]
    async fn create_and_validate_key() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();

        let user = setup_user(&pool).await;
        let (plaintext, key) = create(&pool, user.id, "test-key").await.unwrap();

        assert!(plaintext.starts_with("sk_"));
        assert_eq!(key.name, "test-key");
        assert_eq!(key.user_id, user.id);
        assert!(key.revoked_at.is_none());

        let validated = validate(&pool, &plaintext).await.unwrap();
        assert!(validated.is_some());
        assert_eq!(validated.unwrap().id, key.id);
    }

    #[tokio::test]
    #[ignore = "requires database"]
    async fn validate_invalid_key() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();

        let result = validate(&pool, "sk_totally_bogus_key").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    #[ignore = "requires database"]
    async fn revoke_key_prevents_validation() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();

        let user = setup_user(&pool).await;
        let (plaintext, key) = create(&pool, user.id, "revoke-test").await.unwrap();

        let revoked = revoke(&pool, key.id, user.id).await.unwrap();
        assert!(revoked);

        let validated = validate(&pool, &plaintext).await.unwrap();
        assert!(validated.is_none());
    }

    #[tokio::test]
    #[ignore = "requires database"]
    async fn list_keys_for_user() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();

        let user = setup_user(&pool).await;
        create(&pool, user.id, "key-1").await.unwrap();
        create(&pool, user.id, "key-2").await.unwrap();

        let keys = list_for_user(&pool, user.id).await.unwrap();
        assert_eq!(keys.len(), 2);
    }

    #[tokio::test]
    #[ignore = "requires database"]
    async fn duplicate_key_name_rejected() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();

        let user = setup_user(&pool).await;
        create(&pool, user.id, "dupe-name").await.unwrap();

        let result = create(&pool, user.id, "dupe-name").await;
        assert!(result.is_err());
    }
}
