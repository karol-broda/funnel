use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use funnel_core::tunnel::id::TunnelId;

use crate::api_client;
use crate::config;
use crate::tunnel::{client::TunnelClient, display::TunnelDisplay, runner};

#[derive(clap::Args)]
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

    let public_url =
        runner::build_public_url(&server_url, &tunnel_id).unwrap_or_else(|| "<unknown>".to_string());

    println!("funnel\n");
    println!("  public url  {public_url}");
    println!("  forwarding  {local_addr}");
    println!("  tunnel id   {tunnel_id}");
    if let Some(ref team_name) = args.team {
        println!("  team        {team_name}");
    }
    println!();

    let display = Arc::new(TunnelDisplay::new());

    let client = TunnelClient::new(
        tunnel_id,
        &server_url,
        local_addr,
        token,
        quic_port,
        args.insecure,
        args.team,
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

#[cfg(test)]
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
}
