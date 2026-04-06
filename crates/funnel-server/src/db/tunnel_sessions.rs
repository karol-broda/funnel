use chrono::{DateTime, Utc};
use ipnetwork::IpNetwork;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TunnelSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tunnel_id: String,
    pub client_ip: Option<IpNetwork>,
    pub connected_at: DateTime<Utc>,
    pub disconnected_at: Option<DateTime<Utc>>,
    pub bytes_in: i64,
    pub bytes_out: i64,
    pub requests: i64,
}

pub async fn create(
    pool: &PgPool,
    user_id: Uuid,
    tunnel_id: &str,
    client_ip: Option<IpNetwork>,
) -> Result<TunnelSession, sqlx::Error> {
    sqlx::query_as::<_, TunnelSession>(
        r#"
        INSERT INTO tunnel_sessions (user_id, tunnel_id, client_ip)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(tunnel_id)
    .bind(client_ip)
    .fetch_one(pool)
    .await
}

pub async fn disconnect(
    pool: &PgPool,
    session_id: Uuid,
    bytes_in: i64,
    bytes_out: i64,
    requests: i64,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE tunnel_sessions
        SET disconnected_at = now(), bytes_in = $2, bytes_out = $3, requests = $4
        WHERE id = $1 AND disconnected_at IS NULL
        "#,
    )
    .bind(session_id)
    .bind(bytes_in)
    .bind(bytes_out)
    .bind(requests)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn list_for_user(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<TunnelSession>, sqlx::Error> {
    sqlx::query_as::<_, TunnelSession>(
        r#"
        SELECT * FROM tunnel_sessions
        WHERE user_id = $1
        ORDER BY connected_at DESC
        LIMIT $2
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn list_active(pool: &PgPool) -> Result<Vec<TunnelSession>, sqlx::Error> {
    sqlx::query_as::<_, TunnelSession>(
        "SELECT * FROM tunnel_sessions WHERE disconnected_at IS NULL ORDER BY connected_at DESC",
    )
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::users;

    async fn setup_user(pool: &PgPool) -> users::User {
        users::create(
            pool,
            users::NewUser {
                email: format!("session-test-{}@example.com", Uuid::now_v7()),
                name: Some("Session Test".into()),
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
    async fn create_and_disconnect_session() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();

        let user = setup_user(&pool).await;
        let ip: IpNetwork = "192.168.1.1".parse::<std::net::IpAddr>().unwrap().into();
        let session = create(&pool, user.id, "test-tunnel", Some(ip))
            .await
            .unwrap();

        assert_eq!(session.tunnel_id, "test-tunnel");
        assert!(session.disconnected_at.is_none());
        assert_eq!(session.bytes_in, 0);
        assert!(session.client_ip.is_some());

        let disconnected = disconnect(&pool, session.id, 1024, 2048, 10)
            .await
            .unwrap();
        assert!(disconnected);
    }

    #[tokio::test]
    #[ignore = "requires database"]
    async fn list_active_sessions() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();

        let user = setup_user(&pool).await;
        let s1 = create(&pool, user.id, "active-1", None).await.unwrap();
        let s2 = create(&pool, user.id, "active-2", None).await.unwrap();

        disconnect(&pool, s1.id, 0, 0, 0).await.unwrap();

        let active = list_active(&pool).await.unwrap();
        let active_ids: Vec<Uuid> = active.iter().map(|s| s.id).collect();
        assert!(active_ids.contains(&s2.id));
        assert!(!active_ids.contains(&s1.id));
    }
}
