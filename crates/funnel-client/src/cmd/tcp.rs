use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use funnel_core::protocol::handshake::TunnelType;
use funnel_core::tunnel::id::TunnelId;

use crate::api_client;
use crate::config;
use crate::tunnel::{
    client::{TunnelClient, TunnelConfig},
    display::TunnelDisplay,
    runner,
};

#[derive(clap::Args)]
#[command(after_long_help = super::examples![
    "funnel tcp 5432  # expose localhost:5432",
    "funnel tcp 5432 --id my-db  # custom tunnel id",
    "funnel tcp 22 --id my-ssh --remote-port 2222  # request specific server port",
    "funnel tcp 5432 --team backend  # associate with team",
])]
pub struct Args {
    /// local port to forward to
    pub port: u16,

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

    /// request a specific port on the server (0 = auto-assign)
    #[arg(long, default_value_t = 0)]
    pub remote_port: u16,
}

pub async fn run(ctx_override: Option<&str>, args: Args) -> anyhow::Result<()> {
    let local_addr = format!("localhost:{}", args.port);

    let cfg = config::load_effective().unwrap_or_default();
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
        let api = api_client::ApiClient::new(&server_url, token.clone());
        let info = api.request(&funnel_core::api::INFO).await?;
        info.quic_port
    };

    let tunnel_id = match args.id {
        Some(raw) => TunnelId::new(raw)?,
        None => TunnelId::generate(),
    };

    let remote_port = if args.remote_port != 0 {
        Some(args.remote_port)
    } else {
        None
    };

    println!("funnel tcp\n");
    println!("  forwarding  {local_addr}");
    println!("  tunnel id   {tunnel_id}");
    if let Some(ref team_name) = args.team {
        println!("  team        {team_name}");
    }
    if let Some(rp) = remote_port {
        println!("  remote port {rp} (requested)");
    }
    println!();

    let display = Arc::new(TunnelDisplay::new());

    let client = TunnelClient::new(TunnelConfig {
        tunnel_id,
        server_url,
        local_addr,
        tunnel_type: TunnelType::Stream,
        token,
        quic_port,
        insecure: args.insecure,
        team: args.team,
        remote_port,
        inspector: None,
    })?;

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
