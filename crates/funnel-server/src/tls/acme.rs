use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use instant_acme::{
    Account, AccountCredentials, AuthorizationStatus, ChallengeType, Identifier, NewAccount,
    NewOrder, RetryPolicy,
};
use rustls::sign::CertifiedKey;
use x509_parser::prelude::*;

use super::provider::DnsChallenger;

const POLL_TIMEOUT: Duration = Duration::from_mins(5);
const DNS_PROPAGATION_DELAY: Duration = Duration::from_secs(10);

pub async fn load_or_create_account(
    cert_dir: &Path,
    email: &str,
    staging: bool,
) -> Result<Account> {
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

pub async fn order_certificate(
    account: &Account,
    domain: &str,
    provider: &Arc<dyn DnsChallenger>,
) -> Result<(String, String)> {
    let identifiers = vec![Identifier::Dns(domain.to_string())];
    let mut order = account
        .new_order(&NewOrder::new(&identifiers))
        .await
        .context("failed to create ACME order")?;

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

    let retries = RetryPolicy::new().timeout(POLL_TIMEOUT);

    order
        .poll_ready(&retries)
        .await
        .context("failed waiting for order to become ready")?;

    let private_key_pem = order
        .finalize()
        .await
        .context("failed to finalize ACME order")?;

    let cert_pem = order
        .poll_certificate(&retries)
        .await
        .context("failed to get certificate")?;

    // clean up DNS records (best effort)
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

    Ok((cert_pem, private_key_pem))
}

pub fn load_certified_key(cert_pem: &str, key_pem: &str) -> Result<CertifiedKey> {
    use rustls_pki_types::pem::PemObject;
    use rustls_pki_types::{CertificateDer, PrivateKeyDer};

    let certs: Vec<CertificateDer<'static>> = CertificateDer::pem_slice_iter(cert_pem.as_bytes())
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!(e))
        .context("failed to parse certificate PEM")?;

    let key = PrivateKeyDer::from_pem_slice(key_pem.as_bytes())
        .map_err(|e| anyhow::anyhow!(e))
        .context("failed to parse private key PEM")?;

    let signing_key = rustls::crypto::ring::sign::any_supported_type(&key)
        .map_err(|e| anyhow::anyhow!(e))
        .context("unsupported private key type")?;

    Ok(CertifiedKey::new(certs, signing_key))
}

pub fn parse_cert_expiry(cert_pem: &str) -> Result<SystemTime> {
    use rustls_pki_types::pem::PemObject;
    use rustls_pki_types::CertificateDer;

    let certs: Vec<CertificateDer<'static>> =
        CertificateDer::pem_slice_iter(cert_pem.as_bytes())
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| anyhow::anyhow!(e))?;
    let cert = certs.first().context("no certificate found")?;
    let (_, parsed) = X509Certificate::from_der(cert.as_ref())
        .map_err(|e| anyhow::anyhow!("failed to parse x509 certificate: {e}"))?;

    let not_after = parsed.validity().not_after;
    Ok(not_after.to_datetime().into())
}
