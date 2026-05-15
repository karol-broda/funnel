use funnel_core::api::{self as endpoints, ApiScope, CreateKeyRequest};

use super::api;

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

pub async fn run(server: &str, token: &str, command: Command, json: bool) -> anyhow::Result<()> {
    match command {
        Command::List => list(server, token, json).await,
        Command::Create { name, scopes } => {
            create(server, token, &name, scopes.as_deref(), json).await
        }
        Command::Revoke { id } => revoke(server, token, &id, json).await,
    }
}

fn format_scopes(scopes: &[ApiScope]) -> String {
    scopes
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_scopes(input: &str) -> anyhow::Result<Vec<ApiScope>> {
    input
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            serde_json::from_value(serde_json::Value::String(s.to_string())).map_err(|_| {
                anyhow::anyhow!("invalid scope '{s}', must be 'management' or 'tunnels'")
            })
        })
        .collect()
}

fn format_timestamp(ts: &chrono::DateTime<chrono::Utc>) -> String {
    ts.format("%Y-%m-%dT%H:%M:%S").to_string()
}

pub async fn list(server: &str, token: &str, json: bool) -> anyhow::Result<()> {
    let Some(keys) = api::call(server, token, &endpoints::KEYS_LIST, json).await? else {
        return Ok(());
    };

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
    json: bool,
) -> anyhow::Result<()> {
    let scopes = scopes.map(parse_scopes).transpose()?;

    let body = CreateKeyRequest {
        name: name.to_string(),
        scopes,
        expires_at: None,
    };
    let Some(result) = api::send(server, token, &endpoints::KEYS_CREATE, &body, json).await? else {
        return Ok(());
    };
    println!("created key '{}'", result.info.name);
    println!("  token:  {}", result.key);
    println!("  scopes: {}", format_scopes(&result.info.scopes));
    println!("\nsave this token, it will not be shown again.");

    Ok(())
}

pub async fn revoke(server: &str, token: &str, id: &str, json: bool) -> anyhow::Result<()> {
    let Some(_) = api::call_at(
        server,
        token,
        &endpoints::KEYS_REVOKE,
        &format!("/keys/{id}"),
        json,
    )
    .await?
    else {
        return Ok(());
    };

    println!("revoked key {id}");
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn format_timestamp_strips_subsecond() {
        let ts = "2025-01-15T10:30:00.123456Z"
            .parse::<chrono::DateTime<chrono::Utc>>()
            .unwrap();
        assert_eq!(format_timestamp(&ts), "2025-01-15T10:30:00");
    }

    #[test]
    fn format_scopes_array() {
        let scopes = vec![ApiScope::Management, ApiScope::Tunnels];
        assert_eq!(format_scopes(&scopes), "management, tunnels");
    }

    #[test]
    fn format_scopes_empty() {
        let scopes: Vec<ApiScope> = vec![];
        assert_eq!(format_scopes(&scopes), "");
    }

    #[test]
    fn parse_scopes_valid() {
        let scopes = parse_scopes("management,tunnels").unwrap();
        assert_eq!(scopes, vec![ApiScope::Management, ApiScope::Tunnels]);
    }

    #[test]
    fn parse_scopes_single() {
        assert_eq!(parse_scopes("tunnels").unwrap(), vec![ApiScope::Tunnels]);
    }

    #[test]
    fn parse_scopes_with_whitespace() {
        let scopes = parse_scopes("management , tunnels").unwrap();
        assert_eq!(scopes, vec![ApiScope::Management, ApiScope::Tunnels]);
    }

    #[test]
    fn parse_scopes_rejects_invalid() {
        let err = parse_scopes("management,invalid,tunnels").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("invalid"),
            "error should name the bad scope: {msg}"
        );
    }

    #[test]
    fn parse_scopes_rejects_all_invalid() {
        assert!(parse_scopes("admin,read,write").is_err());
    }

    #[test]
    fn parse_scopes_empty_string_returns_empty() {
        assert!(parse_scopes("").unwrap().is_empty());
    }

    #[test]
    fn parse_scopes_rejects_wrong_case() {
        assert!(parse_scopes("Management,TUNNELS").is_err());
    }

    #[test]
    fn parse_scopes_handles_trailing_comma() {
        let scopes = parse_scopes("management,").unwrap();
        assert_eq!(scopes, vec![ApiScope::Management]);
    }

    #[test]
    fn parse_scopes_handles_leading_comma() {
        let scopes = parse_scopes(",tunnels").unwrap();
        assert_eq!(scopes, vec![ApiScope::Tunnels]);
    }

    #[test]
    fn parse_scopes_deduplication_not_enforced() {
        let scopes = parse_scopes("tunnels,tunnels").unwrap();
        assert_eq!(scopes, vec![ApiScope::Tunnels, ApiScope::Tunnels]);
    }

    #[test]
    fn parse_scopes_error_message_includes_scope_and_hints() {
        let err = parse_scopes("bogus").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("bogus"), "error should include input: {msg}");
        assert!(
            msg.contains("management"),
            "error should hint valid values: {msg}"
        );
    }
}
