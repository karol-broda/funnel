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
    pub role: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct NewUser {
    pub email: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
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

pub async fn create(pool: &PgPool, new_user: NewUser) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r"
        INSERT INTO users (email, name, avatar_url)
        VALUES ($1, $2, $3)
        RETURNING *
        ",
    )
    .bind(&new_user.email)
    .bind(&new_user.name)
    .bind(&new_user.avatar_url)
    .fetch_one(pool)
    .await
}

pub async fn update_profile(
    pool: &PgPool,
    id: Uuid,
    name: Option<&str>,
    avatar_url: Option<&str>,
) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r"
        UPDATE users SET name = $2, avatar_url = $3, updated_at = now()
        WHERE id = $1
        RETURNING *
        ",
    )
    .bind(id)
    .bind(name)
    .bind(avatar_url)
    .fetch_one(pool)
    .await
}
