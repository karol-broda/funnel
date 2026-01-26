use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub provider: String,
    pub provider_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct NewUser {
    pub email: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub provider: String,
    pub provider_id: String,
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(email)
        .fetch_optional(pool)
        .await
}

pub async fn find_by_provider(
    pool: &PgPool,
    provider: &str,
    provider_id: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE provider = $1 AND provider_id = $2",
    )
    .bind(provider)
    .bind(provider_id)
    .fetch_optional(pool)
    .await
}

pub async fn create(pool: &PgPool, new_user: NewUser) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (email, name, avatar_url, provider, provider_id)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(&new_user.email)
    .bind(&new_user.name)
    .bind(&new_user.avatar_url)
    .bind(&new_user.provider)
    .bind(&new_user.provider_id)
    .fetch_one(pool)
    .await
}

/// if a user with the same (provider, provider_id) exists, updates their profile.
/// otherwise creates a new row.
pub async fn upsert_from_oauth(pool: &PgPool, new_user: NewUser) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (email, name, avatar_url, provider, provider_id)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (provider, provider_id)
        DO UPDATE SET
            email = EXCLUDED.email,
            name = EXCLUDED.name,
            avatar_url = EXCLUDED.avatar_url,
            updated_at = now()
        RETURNING *
        "#,
    )
    .bind(&new_user.email)
    .bind(&new_user.name)
    .bind(&new_user.avatar_url)
    .bind(&new_user.provider)
    .bind(&new_user.provider_id)
    .fetch_one(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_new_user() -> NewUser {
        NewUser {
            email: format!("test-{}@example.com", Uuid::now_v7()),
            name: Some("Test User".into()),
            avatar_url: None,
            provider: "github".into(),
            provider_id: Uuid::now_v7().to_string(),
        }
    }

    // run with: DATABASE_URL=postgres://... cargo test -- --ignored
    #[tokio::test]
    #[ignore = "requires database"]
    async fn create_and_find_user() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();

        let new = test_new_user();
        let email = new.email.clone();

        let created = create(&pool, new).await.unwrap();
        assert_eq!(created.email, email);
        assert!(created.id != Uuid::nil());

        let found = find_by_id(&pool, created.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().email, email);
    }

    #[tokio::test]
    #[ignore = "requires database"]
    async fn find_nonexistent_user() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();

        let found = find_by_id(&pool, Uuid::now_v7()).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    #[ignore = "requires database"]
    async fn upsert_creates_then_updates() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();

        let provider_id = Uuid::now_v7().to_string();

        let first = upsert_from_oauth(
            &pool,
            NewUser {
                email: format!("upsert-{}@example.com", Uuid::now_v7()),
                name: Some("First".into()),
                avatar_url: None,
                provider: "github".into(),
                provider_id: provider_id.clone(),
            },
        )
        .await
        .unwrap();

        let second = upsert_from_oauth(
            &pool,
            NewUser {
                email: format!("upsert-updated-{}@example.com", Uuid::now_v7()),
                name: Some("Updated".into()),
                avatar_url: Some("https://example.com/avatar.png".into()),
                provider: "github".into(),
                provider_id: provider_id.clone(),
            },
        )
        .await
        .unwrap();

        assert_eq!(first.id, second.id);
        assert_eq!(second.name.as_deref(), Some("Updated"));
        assert!(second.avatar_url.is_some());
    }
}
