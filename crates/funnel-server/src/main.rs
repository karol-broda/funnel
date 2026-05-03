mod api;
mod app;
mod auth;
mod db;
mod error;
mod metrics;
mod proxy;
mod quic;
mod tls;
mod tunnel;

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::extract::Request;
use axum::response::Redirect;
use clap::Parser;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

use funnel_core::protocol::QUIC_ALPN;

const QUIC_KEEP_ALIVE: Duration = Duration::from_secs(15);
const QUIC_IDLE_TIMEOUT: Duration = Duration::from_mins(1);

#[derive(Parser)]
#[command(name = "funnel-server", about = "Funnel tunnel server")]
struct Cli {
    /// Port to listen on
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// Host to bind to
    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    /// QUIC port for tunnel connections
    #[arg(long, default_value_t = 4433)]
    quic_port: u16,

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
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("failed to install default crypto provider"))?;

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .json()
        .init();

    let cli = Cli::parse();

    tracing::info!(host = %cli.host, port = cli.port, quic_port = cli.quic_port, "starting funnel server");

    let pool = if let Some(ref database_url) = cli.database_url {
        let pool = PgPoolOptions::new()
            .max_connections(cli.db_max_connections)
            .connect(database_url)
            .await?;

        tracing::info!("connected to database");

        sqlx::migrate!("../../migrations").run(&pool).await?;

        tracing::info!("database migrations applied");
        Some(pool)
    } else {
        tracing::info!("no database configured, running in memory only mode");
        None
    };

    if cli.enable_tls {
        run_with_tls(cli, pool).await
    } else {
        run_plain(cli, pool).await
    }
}

async fn run_plain(cli: Cli, pool: Option<sqlx::PgPool>) -> anyhow::Result<()> {
    let metrics_handle = metrics::setup()?;
    let state = Arc::new(app::AppState::new(pool, false));
    let router = app::build_router(Arc::clone(&state), metrics_handle);

    let quic_handle = spawn_quic_listener(&cli.host, cli.quic_port, Arc::clone(&state))?;

    let addr: SocketAddr = format!("{}:{}", cli.host, cli.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!(addr = %addr, "http server listening");

    let http_server = axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    );

    tokio::select! {
        res = http_server => { res?; }
        _ = quic_handle => {}
    };

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

    let rustls_config = tls::setup(
        Path::new(&cli.cert_dir),
        Path::new(config_path),
        email,
        cli.acme_staging,
    )
    .await?;

    let metrics_handle = metrics::setup()?;
    let state = Arc::new(app::AppState::new(pool, true));
    let router = app::build_router(Arc::clone(&state), metrics_handle);

    let quic_handle = spawn_quic_listener(&cli.host, cli.quic_port, Arc::clone(&state))?;

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
        _ = quic_handle => {}
    };

    Ok(())
}

fn spawn_quic_listener(
    host: &str,
    port: u16,
    state: Arc<app::AppState>,
) -> anyhow::Result<tokio::task::JoinHandle<()>> {
    let server_config = build_self_signed_quic_config()?;
    let endpoint = build_quic_endpoint(server_config, host, port)?;

    Ok(tokio::spawn(async move {
        if let Err(e) = quic::listener::run(endpoint, state).await {
            tracing::error!(error = %e, "quic listener failed");
        }
    }))
}

fn build_self_signed_quic_config() -> anyhow::Result<quinn::ServerConfig> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
    let cert_der = cert.cert.der().clone();
    let key_der = rustls::pki_types::PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der());

    let mut server_crypto = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            vec![cert_der],
            rustls::pki_types::PrivateKeyDer::Pkcs8(key_der),
        )?;

    server_crypto.alpn_protocols = vec![QUIC_ALPN.to_vec()];

    let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)?,
    ));

    let mut transport = quinn::TransportConfig::default();
    transport.keep_alive_interval(Some(QUIC_KEEP_ALIVE));
    transport.max_idle_timeout(Some(quinn::IdleTimeout::try_from(QUIC_IDLE_TIMEOUT)?));

    server_config.transport_config(Arc::new(transport));

    Ok(server_config)
}

/// build a quic endpoint, using a dual stack ipv6 socket when the host is a
/// wildcard address so that both ipv4 and ipv6 clients can connect.
fn build_quic_endpoint(
    config: quinn::ServerConfig,
    host: &str,
    port: u16,
) -> anyhow::Result<quinn::Endpoint> {
    let addr: SocketAddr = format!("{host}:{port}").parse()?;

    // when binding to 0.0.0.0, also bind the ipv6 wildcard so dual stack works
    if addr == SocketAddr::from(([0, 0, 0, 0], port)) {
        let v6_addr: SocketAddr = format!("[::]:{port}").parse()?;
        let socket = std::net::UdpSocket::bind(v6_addr)?;
        let runtime = quinn::default_runtime()
            .ok_or_else(|| anyhow::anyhow!("no async runtime available for quic endpoint"))?;
        Ok(quinn::Endpoint::new(
            quinn::EndpointConfig::default(),
            Some(config),
            socket,
            runtime,
        )?)
    } else {
        Ok(quinn::Endpoint::server(config, addr)?)
    }
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
            .map_or("/", axum::http::uri::PathAndQuery::as_str);

        let location = if tls_port == 443 {
            format!("https://{host_without_port}{path}")
        } else {
            format!("https://{host_without_port}:{tls_port}{path}")
        };
        Redirect::permanent(&location)
    })
}
