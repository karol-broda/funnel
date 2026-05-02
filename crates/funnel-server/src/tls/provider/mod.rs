mod cloudflare;
mod exec;
mod route53;

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::{Context, Result};

use self::cloudflare::CloudflareProvider;
use self::exec::ExecProvider;
use self::route53::Route53Provider;
use super::config::ProviderConfig;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait DnsChallenger: Send + Sync {
    fn present(&self, record_name: &str, value: &str) -> BoxFuture<'_, Result<()>>;
    fn cleanup(&self, record_name: &str, value: &str) -> BoxFuture<'_, Result<()>>;
}

/// maps domains to their DNS challenge providers, walking the domain hierarchy for lookups
pub struct ProviderMux {
    providers: HashMap<String, Arc<dyn DnsChallenger>>,
}

impl ProviderMux {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    pub async fn from_config(config: &ProviderConfig) -> Result<Self> {
        let mut mux = Self::new();

        for entry in &config.providers {
            let provider: Arc<dyn DnsChallenger> = match entry.provider_type.as_str() {
                "cloudflare" => {
                    let api_token = entry
                        .config
                        .get("api_token")
                        .context("cloudflare provider requires 'api_token' in config")?;
                    Arc::new(CloudflareProvider::new(api_token.clone()))
                }
                "route53" => {
                    let region = entry.config.get("region").cloned();
                    Arc::new(Route53Provider::new(region).await)
                }
                "exec" => {
                    let command = entry
                        .config
                        .get("command")
                        .context("exec provider requires 'command' in config")?;
                    Arc::new(ExecProvider::new(command.clone()))
                }
                other => anyhow::bail!("unsupported DNS provider type: {other}"),
            };

            for domain in &entry.domains {
                mux.add(domain.clone(), Arc::clone(&provider));
            }
        }

        Ok(mux)
    }

    pub fn add(&mut self, domain: String, provider: Arc<dyn DnsChallenger>) {
        self.providers.insert(domain, provider);
    }

    pub fn find(&self, domain: &str) -> Option<Arc<dyn DnsChallenger>> {
        let parts: Vec<&str> = domain.split('.').collect();
        for i in 0..parts.len() {
            let candidate = parts[i..].join(".");
            if let Some(provider) = self.providers.get(&candidate) {
                return Some(Arc::clone(provider));
            }
        }
        None
    }

    pub fn domains(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
}
