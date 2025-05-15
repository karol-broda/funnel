use std::sync::Arc;

use chrono::Utc;
use ipnetwork::IpNetwork;
use turso::Database;
use uuid::Uuid;

use super::{format_dt, map_err, parse_dt, parse_optional_dt, parse_uuid};
use crate::db::tunnel_sessions::TunnelSession;
use crate::store::StoreError;
use crate::store::session_recorder::SessionRecorder;

pub struct TursoSessionRecorder {
    db: Arc<Database>,
}

impl TursoSessionRecorder {
    pub const fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

fn row_to_session(row: &turso::Row) -> Result<TunnelSession, StoreError> {
    let ip_str: Option<String> = row.get::<Option<String>>(3).map_err(|e| map_err(&e))?;
    let client_ip = match ip_str {
        Some(s) if !s.is_empty() => Some(
            s.parse::<IpNetwork>()
                .map_err(|e| StoreError::Other(format!("invalid ip: {e}")))?,
        ),
        _ => None,
    };
    Ok(TunnelSession {
        id: parse_uuid(&row.get::<String>(0).map_err(|e| map_err(&e))?)?,
        user_id: parse_uuid(&row.get::<String>(1).map_err(|e| map_err(&e))?)?,
        tunnel_id: row.get::<String>(2).map_err(|e| map_err(&e))?,
        client_ip,
        connected_at: parse_dt(&row.get::<String>(4).map_err(|e| map_err(&e))?)?,
        disconnected_at: parse_optional_dt(row.get::<Option<String>>(5).map_err(|e| map_err(&e))?)?,
        bytes_in: row.get::<i64>(6).map_err(|e| map_err(&e))?,
        bytes_out: row.get::<i64>(7).map_err(|e| map_err(&e))?,
        requests: row.get::<i64>(8).map_err(|e| map_err(&e))?,
    })
}

#[async_trait::async_trait]
impl SessionRecorder for TursoSessionRecorder {
    async fn record_connect(
        &self,
        user_id: Uuid,
        tunnel_id: &str,
        client_ip: Option<IpNetwork>,
    ) -> Result<TunnelSession, StoreError> {
        let conn = self.db.connect().map_err(|e| map_err(&e))?;
        let id = Uuid::now_v7();
        let now = Utc::now();
        let ip_str = client_ip.map(|ip| ip.to_string()).unwrap_or_default();

        conn.execute(
            "INSERT INTO tunnel_sessions (id, user_id, tunnel_id, client_ip, connected_at) VALUES (?, ?, ?, ?, ?)",
            turso::params![
                id.to_string(),
                user_id.to_string(),
                tunnel_id.to_string(),
                ip_str,
                format_dt(now)
            ],
        )
        .await
        .map_err(|e| map_err(&e))?;

        Ok(TunnelSession {
            id,
            user_id,
            tunnel_id: tunnel_id.to_string(),
            client_ip,
            connected_at: now,
            disconnected_at: None,
            bytes_in: 0,
            bytes_out: 0,
            requests: 0,
        })
    }

    async fn record_disconnect(
        &self,
        session_id: Uuid,
        bytes_in: i64,
        bytes_out: i64,
        requests: i64,
    ) -> Result<bool, StoreError> {
        let conn = self.db.connect().map_err(|e| map_err(&e))?;
        let now = format_dt(Utc::now());
        let rows_affected = conn
            .execute(
                "UPDATE tunnel_sessions SET disconnected_at = ?, bytes_in = ?, bytes_out = ?, requests = ? WHERE id = ? AND disconnected_at IS NULL",
                turso::params![now, bytes_in, bytes_out, requests, session_id.to_string()],
            )
            .await
            .map_err(|e| map_err(&e))?;
        Ok(rows_affected > 0)
    }

    async fn list_active(&self) -> Result<Vec<TunnelSession>, StoreError> {
        let conn = self.db.connect().map_err(|e| map_err(&e))?;
        let mut rows = conn
            .query(
                "SELECT id, user_id, tunnel_id, client_ip, connected_at, disconnected_at, bytes_in, bytes_out, requests FROM tunnel_sessions WHERE disconnected_at IS NULL ORDER BY connected_at DESC",
                (),
            )
            .await
            .map_err(|e| map_err(&e))?;
        let mut sessions = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| map_err(&e))? {
            sessions.push(row_to_session(&row)?);
        }
        Ok(sessions)
    }

    async fn list_for_user(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<TunnelSession>, StoreError> {
        let conn = self.db.connect().map_err(|e| map_err(&e))?;
        let mut rows = conn
            .query(
                "SELECT id, user_id, tunnel_id, client_ip, connected_at, disconnected_at, bytes_in, bytes_out, requests FROM tunnel_sessions WHERE user_id = ? ORDER BY connected_at DESC LIMIT ?",
                turso::params![user_id.to_string(), limit],
            )
            .await
            .map_err(|e| map_err(&e))?;
        let mut sessions = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| map_err(&e))? {
            sessions.push(row_to_session(&row)?);
        }
        Ok(sessions)
    }

    async fn list_all(&self, limit: i64) -> Result<Vec<TunnelSession>, StoreError> {
        let conn = self.db.connect().map_err(|e| map_err(&e))?;
        let mut rows = conn
            .query(
                "SELECT id, user_id, tunnel_id, client_ip, connected_at, disconnected_at, bytes_in, bytes_out, requests FROM tunnel_sessions ORDER BY connected_at DESC LIMIT ?",
                turso::params![limit],
            )
            .await
            .map_err(|e| map_err(&e))?;
        let mut sessions = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| map_err(&e))? {
            sessions.push(row_to_session(&row)?);
        }
        Ok(sessions)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::db::users::NewUser;
    use crate::store::turso::open;
    use crate::store::turso::user_store::TursoUserStore;
    use crate::store::user_store::UserStore;

    async fn setup() -> (TursoSessionRecorder, Uuid) {
        let db = open(":memory:")
            .await
            .unwrap_or_else(|e| panic!("open: {e}"));
        let user_store = TursoUserStore::new(Arc::clone(&db));
        let user = user_store
            .create(NewUser {
                email: format!("sess-{}@test.com", Uuid::now_v7()),
                name: None,
                avatar_url: None,
            })
            .await
            .unwrap_or_else(|e| panic!("create user: {e}"));
        (TursoSessionRecorder::new(db), user.id)
    }

    #[tokio::test]
    async fn record_connect_and_list_active() {
        let (store, uid) = setup().await;

        let session = store.record_connect(uid, "tunnel-1", None).await.unwrap();
        assert_eq!(session.tunnel_id, "tunnel-1");
        assert_eq!(session.user_id, uid);
        assert!(session.disconnected_at.is_none());

        let active = store.list_active().await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, session.id);
    }

    #[tokio::test]
    async fn record_disconnect_removes_from_active() {
        let (store, uid) = setup().await;

        let session = store.record_connect(uid, "tunnel-2", None).await.unwrap();
        let disconnected = store
            .record_disconnect(session.id, 100, 200, 5)
            .await
            .unwrap();
        assert!(disconnected);

        let active = store.list_active().await.unwrap();
        assert!(active.is_empty());
    }

    #[tokio::test]
    async fn record_disconnect_updates_stats() {
        let (store, uid) = setup().await;

        let session = store.record_connect(uid, "tunnel-3", None).await.unwrap();
        store
            .record_disconnect(session.id, 1024, 2048, 10)
            .await
            .unwrap();

        let all = store.list_for_user(uid, 100).await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].bytes_in, 1024);
        assert_eq!(all[0].bytes_out, 2048);
        assert_eq!(all[0].requests, 10);
        assert!(all[0].disconnected_at.is_some());
    }

    #[tokio::test]
    async fn disconnect_nonexistent_returns_false() {
        let (store, _uid) = setup().await;
        let result = store
            .record_disconnect(Uuid::now_v7(), 0, 0, 0)
            .await
            .unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn disconnect_already_disconnected_returns_false() {
        let (store, uid) = setup().await;

        let session = store.record_connect(uid, "tunnel-4", None).await.unwrap();
        store.record_disconnect(session.id, 0, 0, 0).await.unwrap();

        let again = store.record_disconnect(session.id, 0, 0, 0).await.unwrap();
        assert!(!again);
    }

    #[tokio::test]
    async fn list_for_user_respects_limit() {
        let (store, uid) = setup().await;

        for i in 0..5 {
            store
                .record_connect(uid, &format!("tunnel-{i}"), None)
                .await
                .unwrap();
        }

        let limited = store.list_for_user(uid, 3).await.unwrap();
        assert_eq!(limited.len(), 3);
    }

    #[tokio::test]
    async fn list_for_user_filters_by_user() {
        let db = open(":memory:")
            .await
            .unwrap_or_else(|e| panic!("open: {e}"));
        let user_store = TursoUserStore::new(Arc::clone(&db));
        let u1 = user_store
            .create(NewUser {
                email: format!("u1-{}@t.com", Uuid::now_v7()),
                name: None,
                avatar_url: None,
            })
            .await
            .unwrap();
        let u2 = user_store
            .create(NewUser {
                email: format!("u2-{}@t.com", Uuid::now_v7()),
                name: None,
                avatar_url: None,
            })
            .await
            .unwrap();
        let store = TursoSessionRecorder::new(db);

        store.record_connect(u1.id, "a", None).await.unwrap();
        store.record_connect(u2.id, "b", None).await.unwrap();
        store.record_connect(u1.id, "c", None).await.unwrap();

        let list = store.list_for_user(u1.id, 100).await.unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().all(|s| s.user_id == u1.id));
    }
}
