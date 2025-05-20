use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use funnel_core::protocol::handshake::{AccessControl, TunnelType};
use funnel_core::tunnel::id::TunnelId;

use crate::api_client;
use crate::config;
use crate::tunnel::{
    client::{TunnelClient, TunnelOptions},
    display::TunnelDisplay,
    runner,
};

#[derive(clap::Args)]
#[command(after_long_help = super::examples![
    "funnel http 3000  # localhost:3000",
    "funnel http 3000 --id my-app  # custom subdomain",
    "funnel http 127.0.0.1:8080 --server https://tunnel.example.com  # explicit server",
    "funnel http 3000 --team backend  # associate with team",
    "funnel http 3000 --auth admin:secret  # require http basic auth",
    "funnel http 3000 --allow-ip 10.0.0.0/8  # restrict by client ip",
    "funnel http 3000 --expires 2h  # auto close after a duration",
])]
pub struct Args {
    /// local address or port to forward to (e.g. "3000" or "localhost:3000")
    pub address: String,

    /// tunnel server url (overrides config)
    #[arg(short, long)]
    pub server: Option<String>,

    /// tunnel id (subdomain), generated if omitted
    #[arg(short, long)]
    pub id: Option<String>,

    /// authentication token (overrides config)
    #[arg(short, long)]
    pub token: Option<String>,

    /// quic port on the server (overrides discovery)
    #[arg(long)]
    pub quic_port: Option<u16>,

    /// skip tls certificate verification (for development)
    #[arg(long)]
    pub insecure: bool,

    /// associate tunnel with a team
    #[arg(long)]
    pub team: Option<String>,

    /// require http basic auth for incoming requests (format: user:pass)
    #[arg(long, value_name = "USER:PASS")]
    pub auth: Option<String>,

    /// restrict access to client ip ranges in cidr notation (repeatable)
    #[arg(long, value_name = "CIDR")]
    pub allow_ip: Vec<String>,

    /// automatically close the tunnel after a duration (e.g. 90s, 30m, 2h, 1d)
    #[arg(long, value_name = "DURATION")]
    pub expires: Option<String>,
}

pub async fn run(ctx_override: Option<&str>, args: Args) -> anyhow::Result<()> {
    let local_addr = normalize_address(&args.address);

    let cfg = config::load().unwrap_or_default();
    let resolved = config::resolve(&cfg, ctx_override).ok();

    let server_url = args
        .server
        .or_else(|| resolved.as_ref().map(|r| r.server.clone()))
        .filter(|s| !s.is_empty())
        .map(|s| {
            if s.contains("://") {
                s
            } else {
                format!("http://{s}")
            }
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no server configured. use --server or run: funnel context create default --server <url>"
            )
        })?;

    let token = args
        .token
        .or_else(|| resolved.as_ref().and_then(|r| r.token.clone()));

    let quic_port = if let Some(port) = args.quic_port {
        port
    } else {
        let client = api_client::ApiClient::new(&server_url, token.clone());
        let info = client.request(&funnel_core::api::INFO).await?;
        info.quic_port
    };

    let tunnel_id = match args.id {
        Some(raw) => TunnelId::new(raw)?,
        None => TunnelId::generate(),
    };

    let access = build_access_control(args.auth, args.allow_ip, args.expires.as_deref())?;

    let public_url = runner::build_public_url(&server_url, &tunnel_id)
        .unwrap_or_else(|| "<unknown>".to_string());

    println!("funnel\n");
    println!("  public url  {public_url}");
    println!("  forwarding  {local_addr}");
    println!("  tunnel id   {tunnel_id}");
    if let Some(ref team_name) = args.team {
        println!("  team        {team_name}");
    }
    if let Some(ref access) = access {
        if access.basic_auth.is_some() {
            println!("  basic auth  enabled");
        }
        if !access.allow_ip.is_empty() {
            println!("  allow ip    {}", access.allow_ip.join(", "));
        }
        if let Some(ref expires) = args.expires {
            println!("  expires     {expires}");
        }
    }
    println!();

    let display = Arc::new(TunnelDisplay::new());

    let client = TunnelClient::new(
        &server_url,
        TunnelOptions {
            tunnel_id,
            local_addr,
            tunnel_type: TunnelType::Http,
            token,
            quic_port,
            insecure: args.insecure,
            team: args.team,
            remote_port: None,
            access,
        },
    )?;

    let shutdown = CancellationToken::new();
    let shutdown_signal = shutdown.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        shutdown_signal.cancel();
    });

    runner::run(&client, shutdown, &display).await;

    display.finish();

    Ok(())
}

fn normalize_address(addr: &str) -> String {
    if addr.contains(':') {
        addr.to_string()
    } else {
        format!("localhost:{addr}")
    }
}

fn build_access_control(
    auth: Option<String>,
    allow_ip: Vec<String>,
    expires: Option<&str>,
) -> anyhow::Result<Option<AccessControl>> {
    if auth.as_ref().is_some_and(|creds| !creds.contains(':')) {
        anyhow::bail!("--auth must be in user:pass format");
    }

    let expires_secs = match expires {
        Some(raw) => Some(parse_duration(raw)?),
        None => None,
    };

    let access = AccessControl {
        basic_auth: auth,
        allow_ip,
        expires_secs,
    };

    if access.is_empty() {
        Ok(None)
    } else {
        Ok(Some(access))
    }
}

/// parse a human duration like `90s`, `30m`, `2h`, `1d` into seconds.
/// a bare number is treated as seconds.
fn parse_duration(input: &str) -> anyhow::Result<u64> {
    const SECONDS_PER_MINUTE: u64 = 60;
    const SECONDS_PER_HOUR: u64 = 60 * SECONDS_PER_MINUTE;
    const SECONDS_PER_DAY: u64 = 24 * SECONDS_PER_HOUR;

    let trimmed = input.trim();
    let (value, multiplier) = match trimmed.chars().last() {
        Some('s') => (&trimmed[..trimmed.len() - 1], 1),
        Some('m') => (&trimmed[..trimmed.len() - 1], SECONDS_PER_MINUTE),
        Some('h') => (&trimmed[..trimmed.len() - 1], SECONDS_PER_HOUR),
        Some('d') => (&trimmed[..trimmed.len() - 1], SECONDS_PER_DAY),
        _ => (trimmed, 1),
    };

    let amount: u64 = value
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid duration: {input}"))?;

    if amount == 0 {
        anyhow::bail!("duration must be greater than zero");
    }

    amount
        .checked_mul(multiplier)
        .ok_or_else(|| anyhow::anyhow!("duration too large: {input}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn normalize_port_only() {
        assert_eq!(normalize_address("3000"), "localhost:3000");
    }

    #[test]
    fn normalize_full_address() {
        assert_eq!(normalize_address("127.0.0.1:8080"), "127.0.0.1:8080");
    }

    #[test]
    fn parse_duration_units() {
        assert_eq!(parse_duration("90s").unwrap(), 90);
        assert_eq!(parse_duration("30m").unwrap(), 1800);
        assert_eq!(parse_duration("2h").unwrap(), 7200);
        assert_eq!(parse_duration("1d").unwrap(), 86400);
        assert_eq!(parse_duration("45").unwrap(), 45);
    }

    #[test]
    fn parse_duration_rejects_invalid() {
        assert!(parse_duration("0").is_err());
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("10x").is_err());
        assert!(parse_duration("").is_err());
    }

    #[test]
    fn build_access_control_empty_is_none() {
        let access = build_access_control(None, vec![], None).unwrap();
        assert!(access.is_none());
    }

    #[test]
    fn build_access_control_collects_fields() {
        let access = build_access_control(
            Some("admin:secret".into()),
            vec!["10.0.0.0/8".into()],
            Some("2h"),
        )
        .unwrap()
        .expect("access present");
        assert_eq!(access.basic_auth.as_deref(), Some("admin:secret"));
        assert_eq!(access.allow_ip, vec!["10.0.0.0/8".to_string()]);
        assert_eq!(access.expires_secs, Some(7200));
    }

    #[test]
    fn build_access_control_rejects_auth_without_colon() {
        assert!(build_access_control(Some("nopassword".into()), vec![], None).is_err());
    }
}
