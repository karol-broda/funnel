mod config;
mod forwarder;
mod runner;
mod tunnel;

use clap::{Parser, Subcommand};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use funnel_core::tunnel::id::TunnelId;

#[derive(Parser)]
#[command(name = "funnel", about = "tunnel client for exposing local services")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// create an http tunnel
    Http {
        /// local address or port to forward to (e.g. "3000" or "localhost:3000")
        address: String,

        /// tunnel server url (overrides config)
        #[arg(short, long)]
        server: Option<String>,

        /// tunnel id (subdomain), generated if omitted
        #[arg(short, long)]
        id: Option<String>,

        /// inlet configuration to use
        #[arg(long, default_value = "default")]
        inlet: String,

        /// authentication token (overrides config)
        #[arg(short, long)]
        token: Option<String>,

        /// quic port on the server
        #[arg(long, default_value_t = 4433)]
        quic_port: u16,

        /// skip tls certificate verification (for development with self signed certs)
        #[arg(long)]
        insecure: bool,
    },
    /// manage client configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// save authentication token
    SetToken {
        token: String,
        #[arg(long, default_value = "default")]
        inlet: String,
    },
    /// save server url
    SetServer {
        url: String,
        #[arg(long, default_value = "default")]
        inlet: String,
    },
    /// show current configuration
    Show,
    /// show config file path
    Path,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("failed to install default crypto provider"))?;

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Http {
            address,
            server,
            id,
            inlet,
            token,
            quic_port,
            insecure,
        } => run_http(address, server, id, inlet, token, quic_port, insecure).await?,
        Command::Config { action } => run_config(action)?,
    }

    Ok(())
}

async fn run_http(
    address: String,
    server_flag: Option<String>,
    id_flag: Option<String>,
    inlet_name: String,
    token_flag: Option<String>,
    quic_port: u16,
    insecure: bool,
) -> anyhow::Result<()> {
    let local_addr = normalize_address(&address);

    let cfg = match config::load() {
        Ok(cfg) => cfg,
        Err(e) => {
            tracing::warn!(error = %e, "failed to load config, using defaults");
            config::Config::default()
        }
    };
    let inlet = config::get_inlet(&cfg, &inlet_name);

    let server_url = server_flag
        .or_else(|| inlet.map(|i| i.server.clone()))
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
                "no server configured. use --server or run: funnel config set-server <url>"
            )
        })?;

    let token = token_flag.or_else(|| inlet.and_then(|i| i.token.clone()));

    let tunnel_id = match id_flag {
        Some(raw) => TunnelId::new(raw)?,
        None => TunnelId::generate(),
    };

    if let Some(public_url) = runner::build_public_url(&server_url, &tunnel_id) {
        tracing::info!(
            tunnel_id = %tunnel_id,
            local = %local_addr,
            public_url = %public_url,
            "starting tunnel"
        );
    }

    let client = tunnel::TunnelClient::new(
        tunnel_id,
        &server_url,
        local_addr,
        token,
        quic_port,
        insecure,
    )?;

    let shutdown = CancellationToken::new();
    let shutdown_signal = shutdown.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("received shutdown signal");
        shutdown_signal.cancel();
    });

    runner::run(&client, shutdown).await;

    Ok(())
}

fn normalize_address(addr: &str) -> String {
    if addr.contains(':') {
        addr.to_string()
    } else {
        format!("localhost:{addr}")
    }
}

fn run_config(action: ConfigAction) -> anyhow::Result<()> {
    match action {
        ConfigAction::SetToken { token, inlet } => {
            config::set_token(&inlet, &token)?;
            println!("token saved to inlet \"{inlet}\"");
            println!("  config: {}", config::config_path().display());
        }
        ConfigAction::SetServer { url, inlet } => {
            config::set_server(&inlet, &url)?;
            println!("server saved to inlet \"{inlet}\"");
            println!("  config: {}", config::config_path().display());
        }
        ConfigAction::Show => {
            let cfg = config::load()?;
            println!("config: {}\n", config::config_path().display());
            if cfg.inlets.is_empty() {
                println!("no inlets configured.");
            } else {
                for (name, inlet) in &cfg.inlets {
                    println!("[{name}]");
                    if !inlet.server.is_empty() {
                        println!("  server: {}", inlet.server);
                    }
                    if let Some(domain) = &inlet.domain {
                        println!("  domain: {domain}");
                    }
                    if let Some(token) = &inlet.token {
                        let visible = &token[..token.len().min(10)];
                        println!("  token:  {visible}...");
                    }
                    println!();
                }
            }
        }
        ConfigAction::Path => {
            println!("{}", config::config_path().display());
        }
    }
    Ok(())
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
