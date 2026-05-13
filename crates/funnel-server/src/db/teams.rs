use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct Team {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub const TEAM_ROLE_OWNER: &str = "owner";
pub const TEAM_ROLE_MEMBER: &str = "member";

#[derive(Debug, Clone, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct TeamMembership {
    pub id: Uuid,
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

pub async fn create(pool: &PgPool, name: &str) -> Result<Team, sqlx::Error> {
    sqlx::query_as::<_, Team>(
        r"
        INSERT INTO teams (name)
        VALUES ($1)
        RETURNING *
        ",
    )
    .bind(name)
    .fetch_one(pool)
    .await
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Team>, sqlx::Error> {
    sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn find_by_name(pool: &PgPool, name: &str) -> Result<Option<Team>, sqlx::Error> {
    sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE name = $1")
        .bind(name)
        .fetch_optional(pool)
        .await
}

pub async fn list_all(pool: &PgPool) -> Result<Vec<Team>, sqlx::Error> {
    sqlx::query_as::<_, Team>("SELECT * FROM teams ORDER BY name")
        .fetch_all(pool)
        .await
}

pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM teams WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn add_member(
    pool: &PgPool,
    team_id: Uuid,
    user_id: Uuid,
    role: &str,
) -> Result<TeamMembership, sqlx::Error> {
    sqlx::query_as::<_, TeamMembership>(
        r"
        INSERT INTO team_memberships (team_id, user_id, role)
        VALUES ($1, $2, $3)
        RETURNING *
        ",
    )
    .bind(team_id)
    .bind(user_id)
    .bind(role)
    .fetch_one(pool)
    .await
}

pub async fn update_member_role(
    pool: &PgPool,
    team_id: Uuid,
    user_id: Uuid,
    role: &str,
) -> Result<TeamMembership, sqlx::Error> {
    sqlx::query_as::<_, TeamMembership>(
        r"
        UPDATE team_memberships SET role = $3
        WHERE team_id = $1 AND user_id = $2
        RETURNING *
        ",
    )
    .bind(team_id)
    .bind(user_id)
    .bind(role)
    .fetch_one(pool)
    .await
}

pub async fn find_membership(
    pool: &PgPool,
    team_id: Uuid,
    user_id: Uuid,
) -> Result<Option<TeamMembership>, sqlx::Error> {
    sqlx::query_as::<_, TeamMembership>(
        "SELECT * FROM team_memberships WHERE team_id = $1 AND user_id = $2",
    )
    .bind(team_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn count_owners(pool: &PgPool, team_id: Uuid) -> Result<i64, sqlx::Error> {
    let (count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM team_memberships WHERE team_id = $1 AND role = 'owner'",
    )
    .bind(team_id)
    .fetch_one(pool)
    .await?;
    Ok(count)
}

pub async fn remove_member(
    pool: &PgPool,
    team_id: Uuid,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM team_memberships WHERE team_id = $1 AND user_id = $2")
        .bind(team_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn list_members(
    pool: &PgPool,
    team_id: Uuid,
) -> Result<Vec<TeamMembership>, sqlx::Error> {
    sqlx::query_as::<_, TeamMembership>(
        "SELECT * FROM team_memberships WHERE team_id = $1 ORDER BY created_at",
    )
    .bind(team_id)
    .fetch_all(pool)
    .await
}

pub async fn list_teams_for_user(pool: &PgPool, user_id: Uuid) -> Result<Vec<Team>, sqlx::Error> {
    sqlx::query_as::<_, Team>(
        r"
        SELECT t.* FROM teams t
        INNER JOIN team_memberships tm ON t.id = tm.team_id
        WHERE tm.user_id = $1
        ORDER BY t.name
        ",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn get_team_ids_for_user(pool: &PgPool, user_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
    let rows: Vec<(Uuid,)> =
        sqlx::query_as("SELECT team_id FROM team_memberships WHERE user_id = $1")
            .bind(user_id)
            .fetch_all(pool)
            .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

pub async fn is_member(pool: &PgPool, team_id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let row: (bool,) = sqlx::query_as(
        "SELECT EXISTS(SELECT 1 FROM team_memberships WHERE team_id = $1 AND user_id = $2)",
    )
    .bind(team_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}
