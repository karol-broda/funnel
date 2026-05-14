use ipnetwork::IpNetwork;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::tunnel_sessions::{self, TunnelSession};
use crate::store::session_recorder::SessionRecorder;
use crate::store::StoreError;

pub struct PgSessionRecorder {
    pool: PgPool,
}

impl PgSessionRecorder {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl SessionRecorder for PgSessionRecorder {
    async fn record_connect(
        &self,
        user_id: Uuid,
        tunnel_id: &str,
        client_ip: Option<IpNetwork>,
    ) -> Result<TunnelSession, StoreError> {
        Ok(tunnel_sessions::create(&self.pool, user_id, tunnel_id, client_ip).await?)
    }

    async fn record_disconnect(
        &self,
        session_id: Uuid,
        bytes_in: i64,
        bytes_out: i64,
        requests: i64,
    ) -> Result<bool, StoreError> {
        Ok(
            tunnel_sessions::disconnect(&self.pool, session_id, bytes_in, bytes_out, requests)
                .await?,
        )
    }

    async fn list_active(&self) -> Result<Vec<TunnelSession>, StoreError> {
        Ok(tunnel_sessions::list_active(&self.pool).await?)
    }

    async fn list_for_user(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<TunnelSession>, StoreError> {
        Ok(tunnel_sessions::list_for_user(&self.pool, user_id, limit).await?)
    }

    async fn list_all(&self, limit: i64) -> Result<Vec<TunnelSession>, StoreError> {
        Ok(tunnel_sessions::list_all(&self.pool, limit).await?)
    }
}
