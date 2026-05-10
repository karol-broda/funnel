mod auth;
mod config;
mod display;
mod forwarder;
mod keys;
mod runner;
mod sessions;
mod status;
mod teams;
mod tunnel;
mod users;
mod whoami;

use std::sync::Arc;

use clap::{Parser, Subcommand};
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use funnel_core::tunnel::id::TunnelId;

#[derive(Parser)]
#[command(
    name = "funnel",
    about = "expose local services through secure tunnels"
)]
struct Cli {
    /// context to use (overrides current_context in config)
    #[arg(long, short, global = true)]
    context: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// create an http tunnel to a local service
    Http {
        /// local address or port to forward to (e.g. "3000" or "localhost:3000")
        address: String,

        /// tunnel server url (overrides config)
        #[arg(short, long)]
        server: Option<String>,

        /// tunnel id (subdomain), generated if omitted
        #[arg(short, long)]
        id: Option<String>,

        /// authentication token (overrides config)
        #[arg(short, long)]
        token: Option<String>,

        /// quic port on the server (overrides config)
        #[arg(long)]
        quic_port: Option<u16>,

        /// skip tls certificate verification (for development)
        #[arg(long)]
        insecure: bool,

        /// associate tunnel with a team
        #[arg(long)]
        team: Option<String>,
    },
    /// log in via oauth
    Login {
        /// oauth provider name
        #[arg(long, default_value = "github")]
        provider: String,
    },
    /// log out (clear token for current context)
    Logout,
    /// show the currently authenticated user
    Whoami,
    /// show active tunnels on the server
    Status,
    /// manage api keys
    Keys {
        #[command(subcommand)]
        command: KeysCommand,
    },
    /// view tunnel sessions
    Sessions {
        /// show all sessions (admin only)
        #[arg(long)]
        all: bool,

        /// maximum number of sessions to show
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
    /// manage users (admin only)
    Users {
        #[command(subcommand)]
        command: UsersCommand,
    },
    /// manage teams
    Teams {
        #[command(subcommand)]
        command: TeamsCommand,
    },
    /// manage server contexts
    Context {
        #[command(subcommand)]
        command: ContextCommand,
    },
    /// view configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Subcommand)]
enum KeysCommand {
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

#[derive(Subcommand)]
enum UsersCommand {
    /// list all users
    List {
        /// maximum number of users to show
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
    /// set a user's role
    SetRole {
        /// user id
        id: String,
        /// new role (admin or member)
        role: String,
    },
    /// deactivate a user
    Deactivate {
        /// user id
        id: String,
    },
    /// reactivate a user
    Reactivate {
        /// user id
        id: String,
    },
}

#[derive(Subcommand)]
enum TeamsCommand {
    /// list teams
    List,
    /// create a new team
    Create {
        /// team name
        name: String,
    },
    /// delete a team
    Delete {
        /// team id
        id: String,
    },
    /// list team members
    Members {
        /// team id
        id: String,
    },
    /// add a member to a team
    AddMember {
        /// team id
        team_id: String,
        /// user id to add
        user_id: String,
    },
    /// remove a member from a team
    RemoveMember {
        /// team id
        team_id: String,
        /// user id to remove
        user_id: String,
    },
    /// set a member's role in a team
    SetRole {
        /// team id
        team_id: String,
        /// user id
        user_id: String,
        /// role (owner or member)
        role: String,
    },
}

#[derive(Subcommand)]
enum ContextCommand {
    /// list all contexts
    List,
    /// switch to a different context
    Use {
        /// context name to switch to
        name: String,
    },
    /// create a new context
    Create {
        /// context name
        name: String,
        /// server url
        #[arg(long)]
        server: String,
        /// quic port
        #[arg(long)]
        quic_port: Option<u16>,
    },
    /// delete a context
    Delete {
        /// context name to delete
        name: String,
    },
}

#[derive(Subcommand)]
enum ConfigCommand {
    /// show current configuration
    Show,
    /// print config file path
    Path,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("failed to install default crypto provider"))?;

    let cli = Cli::parse();

    let default_filter = if matches!(cli.command, Command::Http { .. }) {
        "error"
    } else {
        "info"
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter)),
        )
        .init();

    let ctx_override = cli.context.as_deref();

    match cli.command {
        Command::Http {
            address,
            server,
            id,
            token,
            quic_port,
            insecure,
            team,
        } => {
            run_http(
                ctx_override,
                address,
                server,
                id,
                token,
                quic_port,
                insecure,
                team,
            )
            .await
        }
        Command::Login { provider } => {
            let cfg = config::load()?;
            let name = ctx_override.unwrap_or(&cfg.current_context).to_string();
            auth::login(&name, &provider).await
        }
        Command::Logout => {
            let cfg = config::load()?;
            let name = ctx_override.unwrap_or(&cfg.current_context).to_string();
            config::clear_token(&name)?;
            println!("logged out from context '{name}'");
            Ok(())
        }
        Command::Whoami => {
            let cfg = config::load()?;
            let (resolved, token) = config::resolve_authenticated(&cfg, ctx_override)?;
            whoami::run(&resolved.server, &token, &resolved.name).await
        }
        Command::Status => {
            let cfg = config::load()?;
            let (resolved, token) = config::resolve_authenticated(&cfg, ctx_override)?;
            status::run(&resolved.server, &token).await
        }
        Command::Keys { command } => {
            let cfg = config::load()?;
            let (resolved, token) = config::resolve_authenticated(&cfg, ctx_override)?;
            match command {
                KeysCommand::List => keys::list(&resolved.server, &token).await,
                KeysCommand::Create { name, scopes } => {
                    keys::create(&resolved.server, &token, &name, scopes.as_deref()).await
                }
                KeysCommand::Revoke { id } => keys::revoke(&resolved.server, &token, &id).await,
            }
        }
        Command::Sessions { all, limit } => {
            let cfg = config::load()?;
            let (resolved, token) = config::resolve_authenticated(&cfg, ctx_override)?;
            sessions::list(&resolved.server, &token, all, limit).await
        }
        Command::Users { command } => {
            let cfg = config::load()?;
            let (resolved, token) = config::resolve_authenticated(&cfg, ctx_override)?;
            match command {
                UsersCommand::List { limit } => {
                    users::list(&resolved.server, &token, limit).await
                }
                UsersCommand::SetRole { id, role } => {
                    users::set_role(&resolved.server, &token, &id, &role).await
                }
                UsersCommand::Deactivate { id } => {
                    users::deactivate(&resolved.server, &token, &id).await
                }
                UsersCommand::Reactivate { id } => {
                    users::reactivate(&resolved.server, &token, &id).await
                }
            }
        }
        Command::Teams { command } => {
            let cfg = config::load()?;
            let (resolved, token) = config::resolve_authenticated(&cfg, ctx_override)?;
            match command {
                TeamsCommand::List => teams::list(&resolved.server, &token).await,
                TeamsCommand::Create { name } => {
                    teams::create(&resolved.server, &token, &name).await
                }
                TeamsCommand::Delete { id } => {
                    teams::delete(&resolved.server, &token, &id).await
                }
                TeamsCommand::Members { id } => {
                    teams::members(&resolved.server, &token, &id).await
                }
                TeamsCommand::AddMember { team_id, user_id } => {
                    teams::add_member(&resolved.server, &token, &team_id, &user_id).await
                }
                TeamsCommand::RemoveMember { team_id, user_id } => {
                    teams::remove_member(&resolved.server, &token, &team_id, &user_id).await
                }
                TeamsCommand::SetRole {
                    team_id,
                    user_id,
                    role,
                } => {
                    teams::set_role(&resolved.server, &token, &team_id, &user_id, &role).await
                }
            }
        }
        Command::Context { command } => run_context(command),
        Command::Config { command } => run_config(&command),
    }
}

#[allow(clippy::fn_params_excessive_bools, clippy::too_many_arguments)]
async fn run_http(
    ctx_override: Option<&str>,
    address: String,
    server_flag: Option<String>,
    id_flag: Option<String>,
    token_flag: Option<String>,
    quic_port_flag: Option<u16>,
    insecure: bool,
    team: Option<String>,
) -> anyhow::Result<()> {
    let local_addr = normalize_address(&address);

    let cfg = config::load().unwrap_or_default();
    let resolved = config::resolve(&cfg, ctx_override).ok();

    let server_url = server_flag
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

    let token = token_flag.or_else(|| resolved.as_ref().and_then(|r| r.token.clone()));
    let quic_port = quic_port_flag.unwrap_or_else(|| {
        resolved
            .as_ref()
            .map_or(config::DEFAULT_QUIC_PORT, |r| r.quic_port)
    });

    let tunnel_id = match id_flag {
        Some(raw) => TunnelId::new(raw)?,
        None => TunnelId::generate(),
    };

    let public_url =
        runner::build_public_url(&server_url, &tunnel_id).unwrap_or_else(|| "<unknown>".to_string());

    println!("funnel\n");
    println!("  public url  {public_url}");
    println!("  forwarding  {local_addr}");
    println!("  tunnel id   {tunnel_id}");
    if let Some(ref team_name) = team {
        println!("  team        {team_name}");
    }
    println!();

    let display = Arc::new(display::TunnelDisplay::new());

    let client = tunnel::TunnelClient::new(
        tunnel_id,
        &server_url,
        local_addr,
        token,
        quic_port,
        insecure,
        team,
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

fn run_context(command: ContextCommand) -> anyhow::Result<()> {
    match command {
        ContextCommand::List => {
            let cfg = config::load()?;
            if cfg.contexts.is_empty() {
                println!("no contexts configured");
                println!("  create one with: funnel context create <name> --server <url>");
                return Ok(());
            }
            for (name, ctx) in &cfg.contexts {
                let marker = if name == &cfg.current_context {
                    " *"
                } else {
                    ""
                };
                let token_status = if ctx.token.is_some() {
                    "authenticated"
                } else {
                    "no token"
                };
                println!("{name}{marker}");
                println!("  server: {}", ctx.server);
                println!("  status: {token_status}");
                if ctx.quic_port != config::DEFAULT_QUIC_PORT {
                    println!("  quic:   {}", ctx.quic_port);
                }
                println!();
            }
            Ok(())
        }
        ContextCommand::Use { name } => {
            config::set_current_context(&name)?;
            println!("switched to context '{name}'");
            Ok(())
        }
        ContextCommand::Create {
            name,
            server,
            quic_port,
        } => {
            config::create_context(&name, &server, quic_port)?;
            println!("created context '{name}' ({server})");
            Ok(())
        }
        ContextCommand::Delete { name } => {
            config::delete_context(&name)?;
            println!("deleted context '{name}'");
            Ok(())
        }
    }
}

fn run_config(command: &ConfigCommand) -> anyhow::Result<()> {
    match command {
        ConfigCommand::Show => {
            let cfg = config::load()?;
            let content = toml::to_string_pretty(&cfg)?;
            println!("# {}\n", config::config_path().display());
            println!("{content}");
            Ok(())
        }
        ConfigCommand::Path => {
            println!("{}", config::config_path().display());
            Ok(())
        }
    }
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
