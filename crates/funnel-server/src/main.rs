mod api;
mod app;
mod auth;
mod db;
mod error;
mod metrics;
mod proxy;
mod quic;
mod store;
mod tls;
mod tunnel;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use clap::Parser;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::EnvFilter;

use auth::generic::{GenericProvider, GenericProviderConfig};
use auth::github::{GitHubProvider, OAuthConfig};
use auth::oauth::{OAuthProvider, OAuthState};
use store::health::UptimeHealthReporter;
use tunnel::manager::TunnelManager;

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
    name = "funnel-server",
    about = "Funnel tunnel server",
    version = build_version(),
)]
struct Cli {
    /// Port to listen on
    #[arg(short, long, default_value_t = 8080, env = "FUNNEL_PORT")]
    port: u16,

    /// Host to bind to
    #[arg(long, default_value = "0.0.0.0", env = "FUNNEL_HOST")]
    host: String,

    /// QUIC port for tunnel connections
    #[arg(long, default_value_t = 4433, env = "FUNNEL_QUIC_PORT")]
    quic_port: u16,

    /// PostgreSQL connection URL
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,

    /// Maximum database connections
    #[arg(long, default_value_t = 10, env = "FUNNEL_DB_MAX_CONNECTIONS")]
    db_max_connections: u32,

    /// Enable TLS/HTTPS
    #[arg(long, env = "FUNNEL_ENABLE_TLS")]
    enable_tls: bool,

    /// TLS port
    #[arg(long, default_value_t = 8443, env = "FUNNEL_TLS_PORT")]
    tls_port: u16,

    /// Certificate storage directory
    #[arg(long, default_value = "./certs", env = "FUNNEL_CERT_DIR")]
    cert_dir: String,

    /// Let's Encrypt email for ACME registration
    #[arg(long, env = "LETSENCRYPT_EMAIL")]
    letsencrypt_email: Option<String>,

    /// Path to DNS providers config JSON
    #[arg(long, env = "DNS_PROVIDERS_CONFIG")]
    dns_providers_config: Option<String>,

    /// Use Let's Encrypt staging environment
    #[arg(long, env = "FUNNEL_ACME_STAGING")]
    acme_staging: bool,

    /// Create a seed API key at startup and print it to stdout
    #[arg(long, env = "FUNNEL_SEED_API_KEY")]
    seed_api_key: bool,

    /// Path to turso/libsql database file (used when no PostgreSQL URL is set)
    #[arg(long, default_value = "./funnel.db", env = "FUNNEL_TURSO_DB_PATH")]
    turso_db_path: String,

    /// Email of the initial admin user (auto promotes on first login)
    #[arg(long, env = "FUNNEL_INITIAL_ADMIN_EMAIL")]
    initial_admin_email: Option<String>,

    /// GitHub OAuth client ID
    #[arg(long, env = "GITHUB_CLIENT_ID")]
    github_client_id: Option<String>,

    /// GitHub OAuth client secret
    #[arg(long, env = "GITHUB_CLIENT_SECRET")]
    github_client_secret: Option<String>,

    /// Base URL of this server (required when OAuth is configured)
    #[arg(long, env = "BASE_URL")]
    base_url: Option<String>,

    /// Generic OAuth provider name (e.g. "gitlab", "google")
    #[arg(long, env = "OAUTH_PROVIDER_NAME")]
    oauth_provider_name: Option<String>,

    /// Generic OAuth client ID
    #[arg(long, env = "OAUTH_CLIENT_ID")]
    oauth_client_id: Option<String>,

    /// Generic OAuth client secret
    #[arg(long, env = "OAUTH_CLIENT_SECRET")]
    oauth_client_secret: Option<String>,

    /// Generic OAuth authorize URL
    #[arg(long, env = "OAUTH_AUTHORIZE_URL")]
    oauth_authorize_url: Option<String>,

    /// Generic OAuth token exchange URL
    #[arg(long, env = "OAUTH_TOKEN_URL")]
    oauth_token_url: Option<String>,

    /// Generic OAuth user info URL
    #[arg(long, env = "OAUTH_USERINFO_URL")]
    oauth_userinfo_url: Option<String>,

    /// Generic OAuth scopes (space separated)
    #[arg(long, env = "OAUTH_SCOPES", default_value = "openid email profile")]
    oauth_scopes: String,

    /// JSON field for user ID in userinfo response
    #[arg(long, env = "OAUTH_ID_FIELD", default_value = "sub")]
    oauth_id_field: String,

    /// JSON field for email in userinfo response
    #[arg(long, env = "OAUTH_EMAIL_FIELD", default_value = "email")]
    oauth_email_field: String,

    /// JSON field for display name in userinfo response
    #[arg(long, env = "OAUTH_NAME_FIELD", default_value = "name")]
    oauth_name_field: String,

    /// JSON field for avatar URL in userinfo response
    #[arg(long, env = "OAUTH_AVATAR_FIELD", default_value = "picture")]
    oauth_avatar_field: String,
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
        .with_writer(std::io::stderr)
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

        let (db_version,): (i32,) =
            sqlx::query_as("SELECT version FROM schema_version ORDER BY applied_at DESC LIMIT 1")
                .fetch_one(&pool)
                .await?;

        let expected = funnel_core::protocol::PROTOCOL_VERSION as i32;
        if db_version != expected {
            anyhow::bail!(
                "schema version mismatch: database is v{db_version} but server expects v{expected}"
            );
        }

        tracing::info!(schema_version = db_version, "schema version verified");
        Some(pool)
    } else {
        tracing::info!(path = %cli.turso_db_path, "no database url configured, using turso");
        None
    };

    if cli.enable_tls {
        run_with_tls(cli, pool).await
    } else {
        run_plain(cli, pool).await
    }
}

fn build_oauth_state(cli: &Cli) -> anyhow::Result<Option<Arc<OAuthState>>> {
    let mut providers: HashMap<String, Arc<dyn OAuthProvider>> = HashMap::new();

    if let (Some(client_id), Some(client_secret)) =
        (&cli.github_client_id, &cli.github_client_secret)
    {
        let provider: Arc<dyn OAuthProvider> = Arc::new(GitHubProvider::new(OAuthConfig {
            client_id: client_id.clone(),
            client_secret: client_secret.clone(),
        }));
        providers.insert("github".to_string(), provider);
        tracing::info!("github oauth configured");
    }

    if let (
        Some(name),
        Some(client_id),
        Some(client_secret),
        Some(authorize_url),
        Some(token_url),
        Some(userinfo_url),
    ) = (
        &cli.oauth_provider_name,
        &cli.oauth_client_id,
        &cli.oauth_client_secret,
        &cli.oauth_authorize_url,
        &cli.oauth_token_url,
        &cli.oauth_userinfo_url,
    ) {
        let provider: Arc<dyn OAuthProvider> =
            Arc::new(GenericProvider::new(GenericProviderConfig {
                name: name.clone(),
                client_id: client_id.clone(),
                client_secret: client_secret.clone(),
                authorize_url: authorize_url.clone(),
                token_url: token_url.clone(),
                userinfo_url: userinfo_url.clone(),
                scopes: cli.oauth_scopes.clone(),
                id_field: cli.oauth_id_field.clone(),
                email_field: cli.oauth_email_field.clone(),
                name_field: cli.oauth_name_field.clone(),
                avatar_field: cli.oauth_avatar_field.clone(),
            }));
        providers.insert(name.clone(), provider);
        tracing::info!(provider = %name, "generic oauth provider configured");
    }

    if providers.is_empty() {
        return Ok(None);
    }

    let base_url = cli
        .base_url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("--base-url is required when OAuth is configured"))?
        .trim_end_matches('/')
        .to_string();

    Ok(Some(Arc::new(OAuthState::new(providers, base_url))))
}

async fn build_state(
    pool: Option<sqlx::PgPool>,
    turso_db_path: &str,
    is_tls: bool,
    oauth_state: Option<Arc<OAuthState>>,
    initial_admin_email: Option<String>,
    quic_port: u16,
) -> anyhow::Result<Arc<app::AppState>> {
    let tunnels = Arc::new(TunnelManager::new());

    if let Some(pool) = pool {
        Ok(Arc::new(app::AppState {
            tunnels,
            api_keys: Arc::new(store::pg::api_key_store::PgApiKeyStore::new(pool.clone())),
            users: Arc::new(store::pg::user_store::PgUserStore::new(pool.clone())),
            accounts: Arc::new(store::pg::account_store::PgAccountStore::new(pool.clone())),
            sessions: Arc::new(store::pg::session_recorder::PgSessionRecorder::new(
                pool.clone(),
            )),
            teams: Arc::new(store::pg::team_store::PgTeamStore::new(pool)),
            health: Arc::new(UptimeHealthReporter::new()),
            is_tls,
            oauth_state,
            initial_admin_email,
            quic_port,
        }))
    } else {
        let db = store::turso::open(turso_db_path)
            .await
            .map_err(|e| anyhow::anyhow!("failed to open turso database: {e}"))?;

        tracing::info!("turso database ready");

        Ok(Arc::new(app::AppState {
            tunnels,
            api_keys: Arc::new(store::turso::api_key_store::TursoApiKeyStore::new(
                Arc::clone(&db),
            )),
            users: Arc::new(store::turso::user_store::TursoUserStore::new(
                Arc::clone(&db),
            )),
            accounts: Arc::new(store::turso::account_store::TursoAccountStore::new(
                Arc::clone(&db),
            )),
            sessions: Arc::new(store::turso::session_recorder::TursoSessionRecorder::new(
                Arc::clone(&db),
            )),
            teams: Arc::new(store::turso::team_store::TursoTeamStore::new(db)),
            health: Arc::new(UptimeHealthReporter::new()),
            is_tls,
            oauth_state,
            initial_admin_email,
            quic_port,
        }))
    }
}

async fn run_plain(cli: Cli, pool: Option<sqlx::PgPool>) -> anyhow::Result<()> {
    let metrics_handle = metrics::setup()?;
    let oauth_state = build_oauth_state(&cli)?;
    let initial_admin_email = cli.initial_admin_email.clone();
    let state = build_state(pool, &cli.turso_db_path, false, oauth_state, initial_admin_email, cli.quic_port).await?;
    let router = app::build_router(Arc::clone(&state), metrics_handle);

    if cli.seed_api_key {
        create_seed_key(&state).await?;
    }

    let quic_handle = quic::config::spawn_listener(&cli.host, cli.quic_port, Arc::clone(&state))?;

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

async fn create_seed_key(state: &Arc<app::AppState>) -> anyhow::Result<()> {
    use crate::db::users::NewUser;

    let seed_email = "system@funnel.local";

    // find or create the seed user
    let user = match state.users.find_by_email(seed_email).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            let u = state
                .users
                .create(NewUser {
                    email: seed_email.to_string(),
                    name: Some("System".to_string()),
                    avatar_url: None,
                })
                .await
                .map_err(|e| anyhow::anyhow!("failed to create seed user: {e}"))?;

            // promote to admin
            state
                .users
                .update_role(u.id, "admin")
                .await
                .map_err(|e| anyhow::anyhow!("failed to promote seed user: {e}"))?
        }
        Err(e) => anyhow::bail!("failed to look up seed user: {e}"),
    };

    let scopes = db::api_keys::default_scopes();
    match state
        .api_keys
        .create(user.id, "seed", &scopes, None)
        .await
    {
        Ok((plaintext, _)) => {
            tracing::info!(seed_api_key = %plaintext, "seed api key created");
        }
        Err(store::StoreError::Conflict(_)) => {
            tracing::info!("seed key already exists, skipping");
        }
        Err(e) => anyhow::bail!("failed to create seed api key: {e}"),
    }

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
    let oauth_state = build_oauth_state(&cli)?;
    let initial_admin_email = cli.initial_admin_email.clone();
    let state = build_state(pool, &cli.turso_db_path, true, oauth_state, initial_admin_email, cli.quic_port).await?;
    let router = app::build_router(Arc::clone(&state), metrics_handle);

    if cli.seed_api_key {
        create_seed_key(&state).await?;
    }

    let quic_handle = quic::config::spawn_listener(&cli.host, cli.quic_port, Arc::clone(&state))?;

    let tls_addr: SocketAddr = format!("{}:{}", cli.host, cli.tls_port).parse()?;
    let http_addr: SocketAddr = format!("{}:{}", cli.host, cli.port).parse()?;

    tracing::info!(tls_addr = %tls_addr, http_addr = %http_addr, "server listening with TLS");

    let https_server = axum_server::bind_rustls(tls_addr, rustls_config)
        .serve(router.into_make_service_with_connect_info::<SocketAddr>());

    let redirect_router = tls::redirect::router(cli.tls_port);
    let http_listener = tokio::net::TcpListener::bind(http_addr).await?;
    let http_server = axum::serve(http_listener, redirect_router.into_make_service());

    tokio::select! {
        res = https_server => { res?; }
        res = http_server => { res?; }
        _ = quic_handle => {}
    };

    Ok(())
}
