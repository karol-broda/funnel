use anyhow::{Context, Result};

use super::{BoxFuture, DnsChallenger};

pub struct ExecProvider {
    command: String,
}

impl ExecProvider {
    pub const fn new(command: String) -> Self {
        Self { command }
    }

    async fn run(&self, action: &str, record_name: &str, value: &str) -> Result<()> {
        let output = tokio::process::Command::new(&self.command)
            .args([action, record_name, value])
            .output()
            .await
            .with_context(|| format!("failed to execute command: {}", self.command))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("command exited with {}: {}", output.status, stderr.trim());
        }
        Ok(())
    }
}

impl DnsChallenger for ExecProvider {
    fn present(&self, record_name: &str, value: &str) -> BoxFuture<'_, Result<()>> {
        let record_name = record_name.to_string();
        let value = value.to_string();
        Box::pin(async move { self.run("present", &record_name, &value).await })
    }

    fn cleanup(&self, record_name: &str, value: &str) -> BoxFuture<'_, Result<()>> {
        let record_name = record_name.to_string();
        let value = value.to_string();
        Box::pin(async move { self.run("cleanup", &record_name, &value).await })
    }
}
