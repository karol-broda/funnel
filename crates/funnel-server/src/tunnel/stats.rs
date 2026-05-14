use std::sync::atomic::{AtomicU64, Ordering};

pub use funnel_core::api::TunnelStatsSnapshot;

pub struct TunnelStats {
    bytes_in: AtomicU64,
    bytes_out: AtomicU64,
    requests: AtomicU64,
}

impl TunnelStats {
    pub const fn new() -> Self {
        Self {
            bytes_in: AtomicU64::new(0),
            bytes_out: AtomicU64::new(0),
            requests: AtomicU64::new(0),
        }
    }

    pub fn add_bytes_in(&self, n: u64) {
        self.bytes_in.fetch_add(n, Ordering::Relaxed);
    }

    pub fn add_bytes_out(&self, n: u64) {
        self.bytes_out.fetch_add(n, Ordering::Relaxed);
    }

    pub fn inc_requests(&self) {
        self.requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> TunnelStatsSnapshot {
        TunnelStatsSnapshot {
            bytes_in: self.bytes_in.load(Ordering::Relaxed),
            bytes_out: self.bytes_out.load(Ordering::Relaxed),
            requests: self.requests.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_bytes_and_requests() {
        let stats = TunnelStats::new();
        stats.add_bytes_in(100);
        stats.add_bytes_in(200);
        stats.add_bytes_out(50);
        stats.inc_requests();
        stats.inc_requests();

        let snap = stats.snapshot();
        assert_eq!(snap.bytes_in, 300);
        assert_eq!(snap.bytes_out, 50);
        assert_eq!(snap.requests, 2);
    }

    #[test]
    fn snapshot_starts_at_zero() {
        let stats = TunnelStats::new();
        let snap = stats.snapshot();
        assert_eq!(snap.bytes_in, 0);
        assert_eq!(snap.bytes_out, 0);
        assert_eq!(snap.requests, 0);
    }
}
