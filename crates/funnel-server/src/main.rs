mod api;
mod app;
mod auth;
mod db;
mod error;
mod proxy;
mod tls;
mod tunnel;
mod ws;

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use axum::extract::Request;
use axum::response::Redirect;
use axum::Router;
use clap::Parser;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "funnel-server", about = "Funnel tunnel server")]
struct Cli {
    /// Port to listen on
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// Host to bind to
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// PostgreSQL connection URL
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,

    /// Maximum database connections
    #[arg(long, default_value_t = 10)]
    db_max_connections: u32,

    /// Enable TLS/HTTPS
    #[arg(long)]
    enable_tls: bool,

    /// TLS port
    #[arg(long, default_value_t = 8443)]
    tls_port: u16,

    /// Certificate storage directory
    #[arg(long, default_value = "./certs")]
    cert_dir: String,

    /// Let's Encrypt email for ACME registration
    #[arg(long, env = "LETSENCRYPT_EMAIL")]
    letsencrypt_email: Option<String>,

    /// Path to DNS providers config JSON
    #[arg(long, env = "DNS_PROVIDERS_CONFIG")]
    dns_providers_config: Option<String>,

    /// Use Let's Encrypt staging environment
    #[arg(long)]
    acme_staging: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .json()
        .init();

    let cli = Cli::parse();

    tracing::info!(host = %cli.host, port = cli.port, "starting funnel server");

    let pool = if let Some(ref database_url) = cli.database_url {
        let pool = PgPoolOptions::new()
            .max_connections(cli.db_max_connections)
            .connect(database_url)
            .await?;

        tracing::info!("connected to database");

        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await?;

        tracing::info!("database migrations applied");
        Some(pool)
    } else {
        tracing::info!("no database configured, running in memory-only mode");
        None
    };

    if cli.enable_tls {
        run_with_tls(cli, pool).await
    } else {
        run_plain(cli, pool).await
    }
}

async fn run_plain(cli: Cli, pool: Option<sqlx::PgPool>) -> anyhow::Result<()> {
    let state = Arc::new(app::AppState::new(pool, false));
    let router = app::build_router(state);

    let addr: SocketAddr = format!("{}:{}", cli.host, cli.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!(addr = %addr, "server listening");

    axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>()).await?;

    Ok(())
}

async fn run_with_tls(cli: Cli, pool: Option<sqlx::PgPool>) -> anyhow::Result<()> {
    let email = cli
        .letsencrypt_email
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("--letsencrypt-email required when TLS is enabled"))?;
    let config_path = cli
        .dns_providers_config
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("--dns-providers-config required when TLS is enabled"))?;

    let rustls_config =
        tls::setup(Path::new(&cli.cert_dir), Path::new(config_path), email, cli.acme_staging)
            .await?;

    let state = Arc::new(app::AppState::new(pool, true));
    let router = app::build_router(state);

    let tls_addr: SocketAddr = format!("{}:{}", cli.host, cli.tls_port).parse()?;
    let http_addr: SocketAddr = format!("{}:{}", cli.host, cli.port).parse()?;

    tracing::info!(tls_addr = %tls_addr, http_addr = %http_addr, "server listening with TLS");

    let https_server = axum_server::bind_rustls(tls_addr, rustls_config)
        .serve(router.into_make_service_with_connect_info::<SocketAddr>());

    let redirect_router = https_redirect_router(cli.tls_port);
    let http_listener = tokio::net::TcpListener::bind(http_addr).await?;
    let http_server = axum::serve(http_listener, redirect_router.into_make_service());

    tokio::select! {
        res = https_server => { res?; }
        res = http_server => { res?; }
    };

    Ok(())
}

fn https_redirect_router(tls_port: u16) -> Router {
    Router::new().fallback(move |request: Request| async move {
        let host = request
            .headers()
            .get("host")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("localhost");
        let host_without_port = host.split(':').next().unwrap_or(host);
        let path = request
            .uri()
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/");

        let location = if tls_port == 443 {
            format!("https://{host_without_port}{path}")
        } else {
            format!("https://{host_without_port}:{tls_port}{path}")
        };
        Redirect::permanent(&location)
    })
}
