use ipnetwork::IpNetwork;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::tunnel_sessions::{self, TunnelSession};
use crate::store::session_recorder::SessionRecorder;
use crate::store::{BoxFuture, StoreError};

pub struct PgSessionRecorder {
    pool: PgPool,
}

impl PgSessionRecorder {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl SessionRecorder for PgSessionRecorder {
    fn record_connect(
        &self,
        user_id: Uuid,
        tunnel_id: &str,
        client_ip: Option<IpNetwork>,
    ) -> BoxFuture<'_, Result<TunnelSession, StoreError>> {
        let tunnel_id = tunnel_id.to_string();
        Box::pin(async move {
            Ok(tunnel_sessions::create(&self.pool, user_id, &tunnel_id, client_ip).await?)
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
            Ok(tunnel_sessions::disconnect(&self.pool, session_id, bytes_in, bytes_out, requests).await?)
        })
    }

    fn list_active(&self) -> BoxFuture<'_, Result<Vec<TunnelSession>, StoreError>> {
        Box::pin(async move {
            Ok(tunnel_sessions::list_active(&self.pool).await?)
        })
    }

    fn list_for_user(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<TunnelSession>, StoreError>> {
        Box::pin(async move {
            Ok(tunnel_sessions::list_for_user(&self.pool, user_id, limit).await?)
        })
    }
}
