use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use lru::LruCache;
use rustls::sign::CertifiedKey;

const CAPACITY: NonZeroUsize = match NonZeroUsize::new(512) {
    Some(v) => v,
    None => panic!("cache size must be non-zero"),
};

pub struct CachedCert {
    pub certified_key: Arc<CertifiedKey>,
    pub not_after: SystemTime,
}

pub struct CertCache {
    inner: Mutex<LruCache<String, CachedCert>>,
}

impl CertCache {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(LruCache::new(CAPACITY)),
        }
    }

    pub fn get(&self, domain: &str) -> Option<(Arc<CertifiedKey>, SystemTime)> {
        let cache = self
            .inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let cached = cache.peek(domain)?;
        let result = (Arc::clone(&cached.certified_key), cached.not_after);
        drop(cache);
        Some(result)
    }

    pub fn put(&self, domain: String, key: CertifiedKey, not_after: SystemTime) {
        let mut cache = self
            .inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        cache.put(
            domain,
            CachedCert {
                certified_key: Arc::new(key),
                not_after,
            },
        );
    }

    pub fn domains_needing_renewal(&self, window: Duration) -> Vec<String> {
        let cache = self
            .inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        cache
            .iter()
            .filter(|(_, cert)| {
                cert.not_after
                    .duration_since(SystemTime::now())
                    .unwrap_or(Duration::ZERO)
                    < window
            })
            .map(|(domain, _)| domain.clone())
            .collect()
    }
}
