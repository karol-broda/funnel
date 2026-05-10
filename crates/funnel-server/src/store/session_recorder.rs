use ipnetwork::IpNetwork;
use uuid::Uuid;

use super::{BoxFuture, StoreError};
use crate::db::tunnel_sessions::TunnelSession;

pub trait SessionRecorder: Send + Sync {
    fn record_connect(
        &self,
        user_id: Uuid,
        tunnel_id: &str,
        client_ip: Option<IpNetwork>,
    ) -> BoxFuture<'_, Result<TunnelSession, StoreError>>;

    fn record_disconnect(
        &self,
        session_id: Uuid,
        bytes_in: i64,
        bytes_out: i64,
        requests: i64,
    ) -> BoxFuture<'_, Result<bool, StoreError>>;

    fn list_active(&self) -> BoxFuture<'_, Result<Vec<TunnelSession>, StoreError>>;

    fn list_for_user(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<TunnelSession>, StoreError>>;
}
