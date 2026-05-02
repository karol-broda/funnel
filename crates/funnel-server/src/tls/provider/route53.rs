use anyhow::{Context, Result};
use aws_sdk_route53::types::{
    Change, ChangeAction, ChangeBatch, ResourceRecord, ResourceRecordSet, RrType,
};

use super::{BoxFuture, DnsChallenger};

const TXT_TTL: i64 = 60;

pub struct Route53Provider {
    client: aws_sdk_route53::Client,
}

impl Route53Provider {
    pub async fn new(region: Option<String>) -> Self {
        let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest());
        if let Some(region) = region {
            loader = loader.region(aws_config::Region::new(region));
        }
        let config = loader.load().await;
        Self {
            client: aws_sdk_route53::Client::new(&config),
        }
    }

    async fn find_hosted_zone(&self, domain: &str) -> Result<String> {
        let parts: Vec<&str> = domain.split('.').collect();
        for i in 0..parts.len().saturating_sub(1) {
            let zone_name = parts[i..].join(".");
            let resp = self
                .client
                .list_hosted_zones_by_name()
                .dns_name(&zone_name)
                .max_items(1)
                .send()
                .await
                .context("failed to list hosted zones")?;

            if let Some(zone) = resp.hosted_zones().first() {
                let normalized_name = zone.name().trim_end_matches('.');
                if normalized_name == zone_name {
                    let zone_id = zone.id().trim_start_matches("/hostedzone/");
                    return Ok(zone_id.to_string());
                }
            }
        }
        anyhow::bail!("no route53 hosted zone found for domain: {domain}")
    }

    async fn change_record(
        &self,
        action: ChangeAction,
        record_name: &str,
        value: &str,
    ) -> Result<()> {
        let zone_id = self.find_hosted_zone(record_name).await?;

        let record = ResourceRecord::builder()
            .value(format!("\"{value}\""))
            .build()
            .context("failed to build resource record")?;

        let record_set = ResourceRecordSet::builder()
            .name(record_name)
            .r#type(RrType::Txt)
            .ttl(TXT_TTL)
            .resource_records(record)
            .build()
            .context("failed to build resource record set")?;

        let change = Change::builder()
            .action(action)
            .resource_record_set(record_set)
            .build()
            .context("failed to build change")?;

        let batch = ChangeBatch::builder()
            .changes(change)
            .build()
            .context("failed to build change batch")?;

        self.client
            .change_resource_record_sets()
            .hosted_zone_id(&zone_id)
            .change_batch(batch)
            .send()
            .await
            .context("failed to change route53 record")?;

        Ok(())
    }
}

impl DnsChallenger for Route53Provider {
    fn present(&self, record_name: &str, value: &str) -> BoxFuture<'_, Result<()>> {
        let record_name = record_name.to_string();
        let value = value.to_string();
        Box::pin(async move {
            self.change_record(ChangeAction::Upsert, &record_name, &value)
                .await
        })
    }

    fn cleanup(&self, record_name: &str, value: &str) -> BoxFuture<'_, Result<()>> {
        let record_name = record_name.to_string();
        let value = value.to_string();
        Box::pin(async move {
            self.change_record(ChangeAction::Delete, &record_name, &value)
                .await
        })
    }
}
