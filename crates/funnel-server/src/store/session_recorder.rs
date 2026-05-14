use ipnetwork::IpNetwork;
use uuid::Uuid;

use super::StoreError;
use crate::db::tunnel_sessions::TunnelSession;

#[async_trait::async_trait]
pub trait SessionRecorder: Send + Sync {
    async fn record_connect(
        &self,
        user_id: Uuid,
        tunnel_id: &str,
        client_ip: Option<IpNetwork>,
    ) -> Result<TunnelSession, StoreError>;

    async fn record_disconnect(
        &self,
        session_id: Uuid,
        bytes_in: i64,
        bytes_out: i64,
        requests: i64,
    ) -> Result<bool, StoreError>;

    #[allow(dead_code)]
    async fn list_active(&self) -> Result<Vec<TunnelSession>, StoreError>;

    async fn list_for_user(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<TunnelSession>, StoreError>;

    async fn list_all(&self, limit: i64) -> Result<Vec<TunnelSession>, StoreError>;
}
