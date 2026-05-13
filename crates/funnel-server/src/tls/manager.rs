use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use instant_acme::Account;
use rustls::server::ResolvesServerCert;
use rustls::sign::CertifiedKey;

use super::acme;
use super::cache::CertCache;
use super::provider::ProviderMux;

const RENEWAL_CHECK_INTERVAL: Duration = Duration::from_hours(1);
const RENEWAL_WINDOW: Duration = Duration::from_hours(720);

pub struct CertificateManager {
    cache: CertCache,
    obtain_lock: tokio::sync::Mutex<()>,
    cert_dir: PathBuf,
    provider_mux: Arc<ProviderMux>,
    account: Account,
}

impl std::fmt::Debug for CertificateManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CertificateManager")
            .field("cert_dir", &self.cert_dir)
            .finish_non_exhaustive()
    }
}

impl CertificateManager {
    pub async fn new(
        cert_dir: PathBuf,
        provider_mux: ProviderMux,
        email: &str,
        staging: bool,
    ) -> Result<Self> {
        tokio::fs::create_dir_all(&cert_dir)
            .await
            .context("failed to create certificate directory")?;

        let account = acme::load_or_create_account(&cert_dir, email, staging).await?;

        Ok(Self {
            cache: CertCache::new(),
            obtain_lock: tokio::sync::Mutex::new(()),
            cert_dir,
            provider_mux: Arc::new(provider_mux),
            account,
        })
    }

    pub async fn preload_certificates(&self) -> Result<()> {
        for domain in self.provider_mux.domains() {
            match self.load_from_disk(&domain).await {
                Ok(Some((certified_key, not_after))) => {
                    tracing::info!(domain = %domain, "loaded certificate from disk");
                    self.cache.put(domain, certified_key, not_after);
                }
                Ok(None) => {
                    tracing::info!(domain = %domain, "no cached certificate, obtaining from ACME");
                    if let Err(e) = self.obtain_certificate(&domain).await {
                        tracing::error!(domain = %domain, error = %e, "failed to obtain certificate");
                    }
                }
                Err(e) => {
                    tracing::warn!(domain = %domain, error = %e, "failed to load cert from disk");
                    if let Err(e) = self.obtain_certificate(&domain).await {
                        tracing::error!(domain = %domain, error = %e, "failed to obtain certificate");
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn obtain_certificate(&self, domain: &str) -> Result<()> {
        let _guard = self.obtain_lock.lock().await;

        // double check cache after acquiring lock
        if let Some((_, not_after)) = self.cache.get(domain)
            && not_after > SystemTime::now() + RENEWAL_WINDOW
        {
            return Ok(());
        }

        let provider = self
            .provider_mux
            .find(domain)
            .context("no DNS provider configured for domain")?;

        tracing::info!(domain = %domain, "ordering certificate from ACME");

        let (cert_pem, key_pem) = acme::order_certificate(&self.account, domain, &provider).await?;

        self.save_to_disk(domain, &cert_pem, &key_pem).await?;

        let certified_key = acme::load_certified_key(&cert_pem, &key_pem)?;
        let not_after = acme::parse_cert_expiry(&cert_pem)?;

        self.cache.put(domain.to_string(), certified_key, not_after);

        tracing::info!(domain = %domain, "certificate obtained and cached");

        Ok(())
    }

    async fn load_from_disk(&self, domain: &str) -> Result<Option<(CertifiedKey, SystemTime)>> {
        let cert_path = self.cert_dir.join(format!("{domain}.crt"));
        let key_path = self.cert_dir.join(format!("{domain}.key"));

        if !cert_path.exists() || !key_path.exists() {
            return Ok(None);
        }

        let cert_pem = tokio::fs::read_to_string(&cert_path).await?;
        let key_pem = tokio::fs::read_to_string(&key_path).await?;

        let certified_key = acme::load_certified_key(&cert_pem, &key_pem)?;
        let not_after = acme::parse_cert_expiry(&cert_pem)?;

        Ok(Some((certified_key, not_after)))
    }

    async fn save_to_disk(&self, domain: &str, cert_pem: &str, key_pem: &str) -> Result<()> {
        let cert_path = self.cert_dir.join(format!("{domain}.crt"));
        let key_path = self.cert_dir.join(format!("{domain}.key"));
        tokio::fs::write(&cert_path, cert_pem).await?;
        tokio::fs::write(&key_path, key_pem).await?;
        Ok(())
    }

    pub fn spawn_renewal_task(self: &Arc<Self>) {
        let manager = Arc::clone(self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(RENEWAL_CHECK_INTERVAL);
            loop {
                interval.tick().await;
                manager.check_renewals().await;
            }
        });
    }

    async fn check_renewals(&self) {
        let domains = self.cache.domains_needing_renewal(RENEWAL_WINDOW);

        for domain in domains {
            tracing::info!(domain = %domain, "renewing certificate");
            if let Err(e) = self.obtain_certificate(&domain).await {
                tracing::error!(domain = %domain, error = %e, "failed to renew certificate");
            }
        }
    }
}

impl ResolvesServerCert for CertificateManager {
    fn resolve(&self, client_hello: rustls::server::ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        let sni = client_hello.server_name()?;
        let (key, not_after) = self.cache.get(sni)?;

        if not_after > SystemTime::now() {
            Some(key)
        } else {
            None
        }
    }
}
