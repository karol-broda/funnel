mod api_client;
mod cmd;
mod config;
mod tunnel;

use clap::{CommandFactory, Parser, Subcommand};
use tracing_subscriber::EnvFilter;

fn build_version() -> &'static str {
    Box::leak(
        format!(
            "{} ({}) protocol v{}",
            env!("CARGO_PKG_VERSION"),
            env!("FUNNEL_GIT_HASH"),
            funnel_core::protocol::PROTOCOL_VERSION,
        )
        .into_boxed_str(),
    )
}

#[derive(Parser)]
#[command(
    name = "funnel",
    about = "expose local services through secure tunnels",
    version = build_version(),
)]
struct Cli {
    /// context to use (overrides current_context in config)
    #[arg(long, short, global = true)]
    context: Option<String>,

    /// output raw json envelope from the api
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// create an http tunnel to a local service
    Http(cmd::http::Args),
    /// log in via oauth
    #[command(after_long_help = cmd::examples![
        "funnel login  # uses github by default",
        "funnel login --provider gitlab  # use a different provider",
    ])]
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
    #[command(after_long_help = cmd::examples![
        "funnel keys list",
        "funnel keys create deploy-key  # full access",
        "funnel keys create ci-runner --scopes tunnels  # tunnels only",
        "funnel keys revoke <id>",
    ])]
    Keys {
        #[command(subcommand)]
        command: cmd::keys::Command,
    },
    /// view tunnel sessions
    #[command(after_long_help = cmd::examples![
        "funnel sessions",
        "funnel sessions --all  # admin: show all users' sessions",
        "funnel sessions --limit 100",
    ])]
    Sessions {
        /// show all sessions (admin only)
        #[arg(long)]
        all: bool,

        /// maximum number of sessions to show
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
    /// manage users (admin only)
    #[command(after_long_help = cmd::examples![
        "funnel users list",
        "funnel users set-role <id> admin  # promote to admin",
        "funnel users deactivate <id>  # revoke access",
        "funnel users reactivate <id>",
    ])]
    Users {
        #[command(subcommand)]
        command: cmd::users::Command,
    },
    /// manage teams
    #[command(after_long_help = cmd::examples![
        "funnel teams create backend",
        "funnel teams members <id>",
        "funnel teams add-member <team_id> <user_id>",
        "funnel teams set-role <team_id> <user_id> owner  # promote to owner",
        "funnel teams remove-member <team_id> <user_id>",
    ])]
    Teams {
        #[command(subcommand)]
        command: cmd::teams::Command,
    },
    /// manage server contexts
    #[command(after_long_help = cmd::examples![
        "funnel context create staging --server https://tunnel.example.com",
        "funnel context use staging  # switch active context",
        "funnel context list",
    ])]
    Context {
        #[command(subcommand)]
        command: cmd::context::Command,
    },
    /// view configuration
    Config {
        #[command(subcommand)]
        command: cmd::config::Command,
    },
    /// generate cli reference as markdown
    #[command(hide = true)]
    GenerateCliReference {
        /// output as mdx with frontmatter
        #[arg(long)]
        mdx: bool,
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
            cmd::whoami::run(&resolved.server, &token, &resolved.name, cli.json).await
        }
        Command::Status => {
            let cfg = config::load()?;
            let (resolved, token) = config::resolve_authenticated(&cfg, ctx)?;
            cmd::status::run(&resolved.server, &token, cli.json).await
        }
        Command::Keys { command } => {
            let cfg = config::load()?;
            let (resolved, token) = config::resolve_authenticated(&cfg, ctx)?;
            cmd::keys::run(&resolved.server, &token, command, cli.json).await
        }
        Command::Sessions { all, limit } => {
            let cfg = config::load()?;
            let (resolved, token) = config::resolve_authenticated(&cfg, ctx)?;
            cmd::sessions::list(&resolved.server, &token, all, limit, cli.json).await
        }
        Command::Users { command } => {
            let cfg = config::load()?;
            let (resolved, token) = config::resolve_authenticated(&cfg, ctx)?;
            cmd::users::run(&resolved.server, &token, command, cli.json).await
        }
        Command::Teams { command } => {
            let cfg = config::load()?;
            let (resolved, token) = config::resolve_authenticated(&cfg, ctx)?;
            cmd::teams::run(&resolved.server, &token, command, cli.json).await
        }
        Command::Context { command } => cmd::context::run(command),
        Command::Config { command } => cmd::config::run(&command),
        Command::GenerateCliReference { mdx } => {
            print!("{}", cmd::cli_reference::generate(&Cli::command(), mdx));
            Ok(())
        }
    }
}
