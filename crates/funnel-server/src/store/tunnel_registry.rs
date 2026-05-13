use std::sync::Arc;

use funnel_core::tunnel::id::TunnelId;

use crate::tunnel::connection::ActiveTunnel;
use crate::tunnel::manager::TunnelInfo;

pub trait TunnelRegistry: Send + Sync {
    fn insert(&self, id: TunnelId, tunnel: Arc<ActiveTunnel>) -> Result<(), Arc<ActiveTunnel>>;
    fn remove(&self, id: &TunnelId) -> Option<Arc<ActiveTunnel>>;
    fn get(&self, id: &TunnelId) -> Option<Arc<ActiveTunnel>>;
    fn list(&self) -> Vec<TunnelInfo>;
    fn count(&self) -> usize;
}
