use anyhow::Result;

use super::{BoxFuture, DnsChallenger};

const API_BASE: &str = "https://api.cloudflare.com/client/v4";
const TXT_TTL: u32 = 120;

pub struct CloudflareProvider {
    client: reqwest::Client,
    api_token: String,
}

impl CloudflareProvider {
    pub fn new(api_token: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_token,
        }
    }

    async fn find_zone_id(&self, domain: &str) -> Result<String> {
        let parts: Vec<&str> = domain.split('.').collect();
        for i in 0..parts.len().saturating_sub(1) {
            let zone_name = parts[i..].join(".");
            let resp: serde_json::Value = self
                .client
                .get(format!("{API_BASE}/zones"))
                .bearer_auth(&self.api_token)
                .query(&[("name", &zone_name)])
                .send()
                .await?
                .json()
                .await?;

            let zone_id = resp["result"]
                .as_array()
                .and_then(|zones| zones.first())
                .and_then(|zone| zone["id"].as_str());

            if let Some(id) = zone_id {
                return Ok(id.to_string());
            }
        }
        anyhow::bail!("no cloudflare zone found for domain: {domain}")
    }

    async fn create_txt_record(&self, zone_id: &str, name: &str, value: &str) -> Result<()> {
        let resp = self
            .client
            .post(format!("{API_BASE}/zones/{zone_id}/dns_records"))
            .bearer_auth(&self.api_token)
            .json(&serde_json::json!({
                "type": "TXT",
                "name": name,
                "content": value,
                "ttl": TXT_TTL
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await?;
            anyhow::bail!("failed to create TXT record: {body}");
        }
        Ok(())
    }

    async fn delete_txt_records(&self, zone_id: &str, name: &str, value: &str) -> Result<()> {
        let resp: serde_json::Value = self
            .client
            .get(format!("{API_BASE}/zones/{zone_id}/dns_records"))
            .bearer_auth(&self.api_token)
            .query(&[("type", "TXT"), ("name", name)])
            .send()
            .await?
            .json()
            .await?;

        let matching_records = resp["result"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|r| r["content"].as_str() == Some(value))
            .filter_map(|r| r["id"].as_str());

        for record_id in matching_records {
            self.client
                .delete(format!(
                    "{API_BASE}/zones/{zone_id}/dns_records/{record_id}"
                ))
                .bearer_auth(&self.api_token)
                .send()
                .await?;
        }
        Ok(())
    }
}

impl DnsChallenger for CloudflareProvider {
    fn present(&self, record_name: &str, value: &str) -> BoxFuture<'_, Result<()>> {
        let record_name = record_name.to_string();
        let value = value.to_string();
        Box::pin(async move {
            let zone_id = self.find_zone_id(&record_name).await?;
            self.create_txt_record(&zone_id, &record_name, &value).await
        })
    }

    fn cleanup(&self, record_name: &str, value: &str) -> BoxFuture<'_, Result<()>> {
        let record_name = record_name.to_string();
        let value = value.to_string();
        Box::pin(async move {
            let zone_id = self.find_zone_id(&record_name).await?;
            self.delete_txt_records(&zone_id, &record_name, &value)
                .await
        })
    }
}
