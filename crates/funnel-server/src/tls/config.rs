use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ProviderConfig {
    pub providers: Vec<DnsProviderEntry>,
}

#[derive(Debug, Deserialize)]
pub struct DnsProviderEntry {
    #[allow(dead_code)]
    pub name: String,
    #[serde(rename = "type")]
    pub provider_type: String,
    pub domains: Vec<String>,
    pub config: HashMap<String, String>,
}

impl ProviderConfig {
    pub async fn load(path: &Path) -> anyhow::Result<Self> {
        let contents = tokio::fs::read_to_string(path).await?;
        let config: ProviderConfig = serde_json::from_str(&contents)?;
        Ok(config)
    }
}
