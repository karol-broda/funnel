mod api_client;
mod cmd;
mod config;
mod tunnel;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

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
    Http(cmd::http::Args),
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
        command: cmd::keys::Command,
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
        command: cmd::users::Command,
    },
    /// manage teams
    Teams {
        #[command(subcommand)]
        command: cmd::teams::Command,
    },
    /// manage server contexts
    Context {
        #[command(subcommand)]
        command: cmd::context::Command,
    },
    /// view configuration
    Config {
        #[command(subcommand)]
        command: cmd::config::Command,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("failed to install default crypto provider"))?;

    let cli = Cli::parse();

    let default_filter = if matches!(cli.command, Command::Http(_)) {
        "error"
    } else {
        "info"
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter)),
        )
        .init();

    let ctx = cli.context.as_deref();

    match cli.command {
        Command::Http(args) => cmd::http::run(ctx, args).await,
        Command::Login { provider } => {
            let cfg = config::load()?;
            let name = ctx.unwrap_or(&cfg.current_context).to_string();
            cmd::auth::login(&name, &provider).await
        }
        Command::Logout => cmd::auth::logout(ctx),
        Command::Whoami => {
            let cfg = config::load()?;
            let (resolved, token) = config::resolve_authenticated(&cfg, ctx)?;
            cmd::whoami::run(&resolved.server, &token, &resolved.name).await
        }
        Command::Status => {
            let cfg = config::load()?;
            let (resolved, token) = config::resolve_authenticated(&cfg, ctx)?;
            cmd::status::run(&resolved.server, &token).await
        }
        Command::Keys { command } => {
            let cfg = config::load()?;
            let (resolved, token) = config::resolve_authenticated(&cfg, ctx)?;
            cmd::keys::run(&resolved.server, &token, command).await
        }
        Command::Sessions { all, limit } => {
            let cfg = config::load()?;
            let (resolved, token) = config::resolve_authenticated(&cfg, ctx)?;
            cmd::sessions::list(&resolved.server, &token, all, limit).await
        }
        Command::Users { command } => {
            let cfg = config::load()?;
            let (resolved, token) = config::resolve_authenticated(&cfg, ctx)?;
            cmd::users::run(&resolved.server, &token, command).await
        }
        Command::Teams { command } => {
            let cfg = config::load()?;
            let (resolved, token) = config::resolve_authenticated(&cfg, ctx)?;
            cmd::teams::run(&resolved.server, &token, command).await
        }
        Command::Context { command } => cmd::context::run(command),
        Command::Config { command } => cmd::config::run(&command),
    }
}
