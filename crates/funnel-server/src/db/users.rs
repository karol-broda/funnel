use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

pub const ROLE_ADMIN: &str = "admin";
pub const ROLE_MEMBER: &str = "member";

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub role: String,
    #[schema(value_type = Object)]
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deactivated_at: Option<DateTime<Utc>>,
}

impl User {
    pub fn is_admin(&self) -> bool {
        self.role == ROLE_ADMIN
    }

    pub const fn is_active(&self) -> bool {
        self.deactivated_at.is_none()
    }
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

pub async fn update_role(pool: &PgPool, id: Uuid, role: &str) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r"
        UPDATE users SET role = $2, updated_at = now()
        WHERE id = $1
        RETURNING *
        ",
    )
    .bind(id)
    .bind(role)
    .fetch_one(pool)
    .await
}

pub async fn deactivate(pool: &PgPool, id: Uuid) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r"
        UPDATE users SET deactivated_at = now(), updated_at = now()
        WHERE id = $1
        RETURNING *
        ",
    )
    .bind(id)
    .fetch_one(pool)
    .await
}

pub async fn reactivate(pool: &PgPool, id: Uuid) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r"
        UPDATE users SET deactivated_at = NULL, updated_at = now()
        WHERE id = $1
        RETURNING *
        ",
    )
    .bind(id)
    .fetch_one(pool)
    .await
}

pub async fn list_all(pool: &PgPool, limit: i64) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        "SELECT * FROM users ORDER BY created_at DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn count(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

pub async fn count_admins(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM users WHERE role = 'admin' AND deactivated_at IS NULL",
    )
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}
