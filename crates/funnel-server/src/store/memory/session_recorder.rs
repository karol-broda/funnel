use std::sync::{PoisonError, RwLock};

use chrono::Utc;
use ipnetwork::IpNetwork;
use uuid::Uuid;

use crate::db::tunnel_sessions::TunnelSession;
use crate::store::session_recorder::SessionRecorder;
use crate::store::{BoxFuture, StoreError};

pub struct InMemorySessionRecorder {
    sessions: RwLock<Vec<TunnelSession>>,
}

impl InMemorySessionRecorder {
    pub const fn new() -> Self {
        Self {
            sessions: RwLock::new(Vec::new()),
        }
    }
}

impl SessionRecorder for InMemorySessionRecorder {
    fn record_connect(
        &self,
        user_id: Uuid,
        tunnel_id: &str,
        client_ip: Option<IpNetwork>,
    ) -> BoxFuture<'_, Result<TunnelSession, StoreError>> {
        let tunnel_id = tunnel_id.to_string();
        Box::pin(async move {
            let session = TunnelSession {
                id: Uuid::now_v7(),
                user_id,
                tunnel_id,
                client_ip,
                connected_at: Utc::now(),
                disconnected_at: None,
                bytes_in: 0,
                bytes_out: 0,
                requests: 0,
            };
            {
                let mut sessions = self
                    .sessions
                    .write()
                    .unwrap_or_else(PoisonError::into_inner);
                sessions.push(session.clone());
            }
            Ok(session)
        })
    }

    fn record_disconnect(
        &self,
        session_id: Uuid,
        bytes_in: i64,
        bytes_out: i64,
        requests: i64,
    ) -> BoxFuture<'_, Result<bool, StoreError>> {
        Box::pin(async move {
            let mut sessions = self
                .sessions
                .write()
                .unwrap_or_else(PoisonError::into_inner);
            if let Some(session) = sessions
                .iter_mut()
                .find(|s| s.id == session_id && s.disconnected_at.is_none())
            {
                session.disconnected_at = Some(Utc::now());
                session.bytes_in = bytes_in;
                session.bytes_out = bytes_out;
                session.requests = requests;
                Ok(true)
            } else {
                Ok(false)
            }
        })
    }

    fn list_active(&self) -> BoxFuture<'_, Result<Vec<TunnelSession>, StoreError>> {
        Box::pin(async move {
            let sessions = self.sessions.read().unwrap_or_else(PoisonError::into_inner);
            Ok(sessions
                .iter()
                .filter(|s| s.disconnected_at.is_none())
                .cloned()
                .collect())
        })
    }

    fn list_for_user(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<TunnelSession>, StoreError>> {
        Box::pin(async move {
            let sessions = self.sessions.read().unwrap_or_else(PoisonError::into_inner);
            let limit = usize::try_from(limit).unwrap_or(usize::MAX);
            Ok(sessions
                .iter()
                .filter(|s| s.user_id == user_id)
                .rev()
                .take(limit)
                .cloned()
                .collect())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user_id() -> Uuid {
        Uuid::now_v7()
    }

    #[tokio::test]
    async fn record_connect_and_list_active() {
        let store = InMemorySessionRecorder::new();
        let uid = user_id();

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
        let store = InMemorySessionRecorder::new();
        let uid = user_id();

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
        let store = InMemorySessionRecorder::new();
        let uid = user_id();

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
        let store = InMemorySessionRecorder::new();
        let result = store
            .record_disconnect(Uuid::now_v7(), 0, 0, 0)
            .await
            .unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn disconnect_already_disconnected_returns_false() {
        let store = InMemorySessionRecorder::new();
        let uid = user_id();

        let session = store.record_connect(uid, "tunnel-4", None).await.unwrap();
        store.record_disconnect(session.id, 0, 0, 0).await.unwrap();

        let again = store.record_disconnect(session.id, 0, 0, 0).await.unwrap();
        assert!(!again);
    }

    #[tokio::test]
    async fn list_for_user_respects_limit() {
        let store = InMemorySessionRecorder::new();
        let uid = user_id();

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
        let store = InMemorySessionRecorder::new();
        let uid1 = user_id();
        let uid2 = user_id();

        store.record_connect(uid1, "a", None).await.unwrap();
        store.record_connect(uid2, "b", None).await.unwrap();
        store.record_connect(uid1, "c", None).await.unwrap();

        let list = store.list_for_user(uid1, 100).await.unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().all(|s| s.user_id == uid1));
    }
}
