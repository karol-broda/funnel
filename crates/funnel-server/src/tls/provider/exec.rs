use anyhow::{Context, Result};

use super::DnsChallenger;

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

#[async_trait::async_trait]
impl DnsChallenger for ExecProvider {
    async fn present(&self, record_name: &str, value: &str) -> Result<()> {
        self.run("present", record_name, value).await
    }

    async fn cleanup(&self, record_name: &str, value: &str) -> Result<()> {
        self.run("cleanup", record_name, value).await
    }
}
