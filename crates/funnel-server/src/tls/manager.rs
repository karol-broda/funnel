use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use instant_acme::{
    Account, AccountCredentials, AuthorizationStatus, ChallengeType, Identifier, NewAccount,
    NewOrder, RetryPolicy,
};
use lru::LruCache;
use rustls::server::ResolvesServerCert;
use rustls::sign::CertifiedKey;
use x509_parser::prelude::*;

use super::provider::ProviderMux;

const LRU_CACHE_SIZE: std::num::NonZeroUsize = std::num::NonZeroUsize::new(512).unwrap();
const RENEWAL_CHECK_INTERVAL: Duration = Duration::from_hours(1);
const RENEWAL_WINDOW: Duration = Duration::from_hours(720);
const ACME_POLL_TIMEOUT: Duration = Duration::from_mins(5);
const DNS_PROPAGATION_DELAY: Duration = Duration::from_secs(10);

struct CachedCert {
    certified_key: Arc<CertifiedKey>,
    not_after: SystemTime,
}

pub struct CertificateManager {
    cache: Mutex<LruCache<String, CachedCert>>,
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

        let account = load_or_create_account(&cert_dir, email, staging).await?;
        let cache = Mutex::new(LruCache::new(LRU_CACHE_SIZE));

        Ok(Self {
            cache,
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
                    let mut cache = self
                        .cache
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    cache.put(
                        domain,
                        CachedCert {
                            certified_key: Arc::new(certified_key),
                            not_after,
                        },
                    );
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
        {
            let cache = self
                .cache
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(cached) = cache.peek(domain)
                && cached.not_after > SystemTime::now() + RENEWAL_WINDOW
            {
                return Ok(());
            }
        }

        let provider = self
            .provider_mux
            .find(domain)
            .context("no DNS provider configured for domain")?;

        tracing::info!(domain = %domain, "ordering certificate from ACME");

        let identifiers = vec![Identifier::Dns(domain.to_string())];
        let mut order = self
            .account
            .new_order(&NewOrder::new(&identifiers))
            .await
            .context("failed to create ACME order")?;

        // process authorizations: present DNS challenges and mark them ready
        struct PendingCleanup {
            record_name: String,
            dns_value: String,
        }
        let mut cleanups = Vec::new();

        {
            let mut authz_stream = order.authorizations();
            while let Some(result) = authz_stream.next().await {
                let mut authz = result.context("failed to get authorization")?;

                if matches!(authz.status, AuthorizationStatus::Valid) {
                    continue;
                }

                let challenge_domain = match authz.identifier().identifier {
                    Identifier::Dns(d) => d.clone(),
                    _ => continue,
                };

                let mut challenge = authz
                    .challenge(ChallengeType::Dns01)
                    .context("no DNS-01 challenge found")?;

                let dns_value = challenge.key_authorization().dns_value();

                let record_name = format!("_acme-challenge.{challenge_domain}");
                provider
                    .present(&record_name, &dns_value)
                    .await
                    .context("failed to present DNS challenge")?;

                tokio::time::sleep(DNS_PROPAGATION_DELAY).await;

                challenge
                    .set_ready()
                    .await
                    .context("failed to set challenge ready")?;

                cleanups.push(PendingCleanup {
                    record_name,
                    dns_value,
                });
            }
        }

        let retries = RetryPolicy::new().timeout(ACME_POLL_TIMEOUT);

        order
            .poll_ready(&retries)
            .await
            .context("failed waiting for order to become ready")?;

        // finalize: generates ECDSA key + CSR internally, returns private key PEM
        let private_key_pem = order
            .finalize()
            .await
            .context("failed to finalize ACME order")?;

        let cert_pem = order
            .poll_certificate(&retries)
            .await
            .context("failed to get certificate")?;

        // save to disk
        let cert_path = self.cert_dir.join(format!("{domain}.crt"));
        let key_path = self.cert_dir.join(format!("{domain}.key"));
        tokio::fs::write(&cert_path, &cert_pem).await?;
        tokio::fs::write(&key_path, &private_key_pem).await?;

        // parse and cache
        let certified_key = load_certified_key(&cert_pem, &private_key_pem)?;
        let not_after = parse_cert_expiry(&cert_pem)?;

        {
            let mut cache = self
                .cache
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            cache.put(
                domain.to_string(),
                CachedCert {
                    certified_key: Arc::new(certified_key),
                    not_after,
                },
            );
        }

        tracing::info!(domain = %domain, "certificate obtained and cached");

        // clean up DNS records (best effort, after everything succeeded)
        for cleanup in &cleanups {
            if let Err(e) = provider
                .cleanup(&cleanup.record_name, &cleanup.dns_value)
                .await
            {
                tracing::warn!(
                    record = %cleanup.record_name,
                    error = %e,
                    "failed to clean up DNS record"
                );
            }
        }

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

        let certified_key = load_certified_key(&cert_pem, &key_pem)?;
        let not_after = parse_cert_expiry(&cert_pem)?;

        Ok(Some((certified_key, not_after)))
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
        let domains_to_renew: Vec<String> = {
            let cache = self
                .cache
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            cache
                .iter()
                .filter(|(_, cert)| {
                    cert.not_after
                        .duration_since(SystemTime::now())
                        .unwrap_or(Duration::ZERO)
                        < RENEWAL_WINDOW
                })
                .map(|(domain, _)| domain.clone())
                .collect()
        };

        for domain in domains_to_renew {
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
        let cache = self
            .cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let cached = cache.peek(sni)?;
        let not_after = cached.not_after;
        let key = Arc::clone(&cached.certified_key);
        drop(cache);

        if not_after > SystemTime::now() {
            Some(key)
        } else {
            None
        }
    }
}

fn load_certified_key(cert_pem: &str, key_pem: &str) -> Result<CertifiedKey> {
    let certs = rustls_pemfile::certs(&mut BufReader::new(cert_pem.as_bytes()))
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to parse certificate PEM")?;

    let key = rustls_pemfile::private_key(&mut BufReader::new(key_pem.as_bytes()))
        .context("failed to parse private key PEM")?
        .context("no private key found")?;

    let signing_key = rustls::crypto::ring::sign::any_supported_type(&key)
        .map_err(|e| anyhow::anyhow!(e))
        .context("unsupported private key type")?;

    Ok(CertifiedKey::new(certs, signing_key))
}

fn parse_cert_expiry(cert_pem: &str) -> Result<SystemTime> {
    let certs: Vec<_> = rustls_pemfile::certs(&mut BufReader::new(cert_pem.as_bytes()))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let cert = certs.first().context("no certificate found")?;
    let (_, parsed) = X509Certificate::from_der(cert.as_ref())
        .map_err(|e| anyhow::anyhow!("failed to parse x509 certificate: {e}"))?;

    let not_after = parsed.validity().not_after;
    Ok(not_after.to_datetime().into())
}

async fn load_or_create_account(cert_dir: &Path, email: &str, staging: bool) -> Result<Account> {
    let account_json_path = cert_dir.join("account.json");

    if account_json_path.exists() {
        let json = tokio::fs::read_to_string(&account_json_path).await?;
        let credentials: AccountCredentials = serde_json::from_str(&json)?;
        let account = Account::builder()
            .context("failed to create account builder")?
            .from_credentials(credentials)
            .await
            .context("failed to load ACME account from credentials")?;
        tracing::info!("loaded existing ACME account");
        return Ok(account);
    }

    let directory_url = if staging {
        instant_acme::LetsEncrypt::Staging.url()
    } else {
        instant_acme::LetsEncrypt::Production.url()
    };

    let (account, credentials) = Account::builder()
        .context("failed to create account builder")?
        .create(
            &NewAccount {
                contact: &[&format!("mailto:{email}")],
                terms_of_service_agreed: true,
                only_return_existing: false,
            },
            directory_url.to_string(),
            None,
        )
        .await
        .context("failed to create ACME account")?;

    let json = serde_json::to_string_pretty(&credentials)?;
    tokio::fs::write(&account_json_path, &json).await?;

    tracing::info!("created new ACME account");
    Ok(account)
}
