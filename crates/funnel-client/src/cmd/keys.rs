use funnel_core::protocol::PROTOCOL_VERSION;
use serde::{Deserialize, Serialize};

#[derive(clap::Subcommand)]
pub enum Command {
    /// list api keys
    List,
    /// create a new api key
    Create {
        /// name for the new key
        name: String,
        /// comma separated scopes (defaults to management,tunnels)
        #[arg(long)]
        scopes: Option<String>,
    },
    /// revoke an api key
    Revoke {
        /// key id to revoke
        id: String,
    },
}

pub async fn run(server: &str, token: &str, command: Command) -> anyhow::Result<()> {
    match command {
        Command::List => list(server, token).await,
        Command::Create { name, scopes } => create(server, token, &name, scopes.as_deref()).await,
        Command::Revoke { id } => revoke(server, token, &id).await,
    }
}

#[derive(Deserialize)]
struct ApiKeyView {
    id: String,
    name: String,
    key_prefix: String,
    scopes: serde_json::Value,
    created_at: String,
}

#[derive(Deserialize)]
struct CreateKeyResponse {
    key: String,
    info: ApiKeyView,
}

#[derive(Serialize)]
struct CreateKeyRequest {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    scopes: Option<Vec<String>>,
}

fn client(token: &str) -> reqwest::Client {
    reqwest::Client::builder()
        .default_headers({
            let mut h = reqwest::header::HeaderMap::new();
            if let Ok(v) = reqwest::header::HeaderValue::from_str(&format!("Bearer {token}")) {
                h.insert(reqwest::header::AUTHORIZATION, v);
            }
            h
        })
        .build()
        .unwrap_or_default()
}

fn format_scopes(scopes: &serde_json::Value) -> String {
    scopes
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

fn format_timestamp(ts: &str) -> &str {
    // trim sub-second precision and timezone suffix for readability
    // "2025-01-15T10:30:00.123456Z" -> "2025-01-15T10:30:00"
    let without_frac = ts.split('.').next().unwrap_or(ts);
    without_frac.strip_suffix('Z').unwrap_or(without_frac)
}

pub async fn list(server: &str, token: &str) -> anyhow::Result<()> {
    let resp = client(token)
        .get(format!("{server}/api/v{PROTOCOL_VERSION}/keys"))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("server returned {status}: {body}");
    }

    let keys: Vec<ApiKeyView> = resp.json().await?;

    if keys.is_empty() {
        println!("no api keys");
        return Ok(());
    }

    for (i, key) in keys.iter().enumerate() {
        println!("{}", key.name);
        println!("  id:      {}", key.id);
        println!("  prefix:  {}...", key.key_prefix);
        println!("  scopes:  {}", format_scopes(&key.scopes));
        println!("  created: {}", format_timestamp(&key.created_at));
        if i < keys.len() - 1 {
            println!();
        }
    }

    if keys.len() > 1 {
        println!("\n{} keys", keys.len());
    }

    Ok(())
}

pub async fn create(
    server: &str,
    token: &str,
    name: &str,
    scopes: Option<&str>,
) -> anyhow::Result<()> {
    let scopes = scopes.map(|s| s.split(',').map(|s| s.trim().to_string()).collect());

    let resp = client(token)
        .post(format!("{server}/api/v{PROTOCOL_VERSION}/keys"))
        .json(&CreateKeyRequest {
            name: name.to_string(),
            scopes,
        })
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("server returned {status}: {body}");
    }

    let result: CreateKeyResponse = resp.json().await?;
    println!("created key '{}'", result.info.name);
    println!("  token:  {}", result.key);
    println!("  scopes: {}", format_scopes(&result.info.scopes));
    println!("\nsave this token, it will not be shown again.");

    Ok(())
}

pub async fn revoke(server: &str, token: &str, id: &str) -> anyhow::Result<()> {
    let resp = client(token)
        .delete(format!("{server}/api/v{PROTOCOL_VERSION}/keys/{id}"))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("server returned {status}: {body}");
    }

    println!("revoked key {id}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_timestamp_trims_fractional() {
        assert_eq!(
            format_timestamp("2025-01-15T10:30:00.123456Z"),
            "2025-01-15T10:30:00"
        );
    }

    #[test]
    fn format_timestamp_handles_no_fractional() {
        assert_eq!(
            format_timestamp("2025-01-15T10:30:00Z"),
            "2025-01-15T10:30:00"
        );
    }

    #[test]
    fn format_timestamp_passthrough() {
        assert_eq!(format_timestamp("unknown"), "unknown");
    }

    #[test]
    fn format_scopes_array() {
        let scopes = serde_json::json!(["management", "tunnels"]);
        assert_eq!(format_scopes(&scopes), "management, tunnels");
    }

    #[test]
    fn format_scopes_empty() {
        let scopes = serde_json::json!([]);
        assert_eq!(format_scopes(&scopes), "");
    }
}
