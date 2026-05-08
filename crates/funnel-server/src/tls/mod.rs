mod acme;
mod cache;
pub mod config;
pub mod manager;
pub mod provider;
pub mod redirect;

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};

pub async fn setup(
    cert_dir: &Path,
    providers_config_path: &Path,
    email: &str,
    acme_staging: bool,
) -> Result<axum_server::tls_rustls::RustlsConfig> {
    let provider_config = config::ProviderConfig::load(providers_config_path).await?;
    let provider_mux = provider::ProviderMux::from_config(&provider_config).await?;

    let cert_manager = Arc::new(
        manager::CertificateManager::new(cert_dir.to_path_buf(), provider_mux, email, acme_staging)
            .await
            .context("failed to create certificate manager")?,
    );

    cert_manager.preload_certificates().await?;
    cert_manager.spawn_renewal_task();

    let server_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(cert_manager as Arc<dyn rustls::server::ResolvesServerCert>);

    Ok(axum_server::tls_rustls::RustlsConfig::from_config(
        Arc::new(server_config),
    ))
}
