mod api;
mod app;
mod auth;
mod db;
mod error;
mod proxy;
mod tls;
mod tunnel;
mod ws;

use std::sync::Arc;

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
    database_url: String,

    /// Maximum database connections
    #[arg(long, default_value_t = 10)]
    db_max_connections: u32,
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

    let pool = PgPoolOptions::new()
        .max_connections(cli.db_max_connections)
        .connect(&cli.database_url)
        .await?;

    tracing::info!("connected to database");

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await?;

    tracing::info!("database migrations applied");

    let state = Arc::new(app::AppState::new(pool));
    let router = app::build_router(state);

    let addr = format!("{}:{}", cli.host, cli.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!(addr = %addr, "server listening");

    axum::serve(listener, router).await?;

    Ok(())
}
