use std::sync::Arc;

use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use serde::Serialize;

use funnel_core::tunnel::TunnelId;

use super::connection::ActiveTunnel;
use super::stats::TunnelStatsSnapshot;

pub struct TunnelManager {
    tunnels: DashMap<TunnelId, Arc<ActiveTunnel>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TunnelInfo {
    pub id: String,
    pub uptime_secs: f64,
    pub stats: TunnelStatsSnapshot,
}

impl TunnelManager {
    pub fn new() -> Self {
        Self {
            tunnels: DashMap::new(),
        }
    }

    pub fn insert(&self, id: TunnelId, tunnel: Arc<ActiveTunnel>) -> Result<(), Arc<ActiveTunnel>> {
        match self.tunnels.entry(id) {
            Entry::Occupied(_) => Err(tunnel),
            Entry::Vacant(e) => {
                e.insert(tunnel);
                Ok(())
            }
        }
    }

    pub fn remove(&self, id: &TunnelId) -> Option<Arc<ActiveTunnel>> {
        self.tunnels.remove(id).map(|(_, t)| t)
    }

    pub fn get(&self, id: &TunnelId) -> Option<Arc<ActiveTunnel>> {
        self.tunnels.get(id).map(|r| Arc::clone(r.value()))
    }

    pub fn exists(&self, id: &TunnelId) -> bool {
        self.tunnels.contains_key(id)
    }

    pub fn list(&self) -> Vec<TunnelInfo> {
        self.tunnels
            .iter()
            .map(|entry| {
                let tunnel = entry.value();
                TunnelInfo {
                    id: entry.key().to_string(),
                    uptime_secs: tunnel.connected_at().elapsed().as_secs_f64(),
                    stats: tunnel.stats(),
                }
            })
            .collect()
    }

    pub fn count(&self) -> usize {
        self.tunnels.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_manager_is_empty() {
        let mgr = TunnelManager::new();
        assert_eq!(mgr.count(), 0);
        assert!(mgr.list().is_empty());
    }

    #[test]
    fn exists_returns_false_for_missing() {
        let mgr = TunnelManager::new();
        let id = TunnelId::new("test-abc").unwrap();
        assert!(!mgr.exists(&id));
    }

    #[test]
    fn get_returns_none_for_missing() {
        let mgr = TunnelManager::new();
        let id = TunnelId::new("test-abc").unwrap();
        assert!(mgr.get(&id).is_none());
    }

    #[test]
    fn remove_returns_none_for_missing() {
        let mgr = TunnelManager::new();
        let id = TunnelId::new("test-abc").unwrap();
        assert!(mgr.remove(&id).is_none());
    }
}
