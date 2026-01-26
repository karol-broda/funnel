use std::sync::Arc;

use axum::Router;
use axum::routing::get;
use sqlx::PgPool;
use tokio::time::Instant;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::api;

pub struct AppState {
    pub db: PgPool,
    pub start_time: Instant,
}

impl AppState {
    pub fn new(db: PgPool) -> Self {
        Self {
            db,
            start_time: Instant::now(),
        }
    }
}

pub fn build_router(state: Arc<AppState>) -> Router {
    let api_routes = Router::new()
        .route("/health", get(api::health::handler))
        .route("/tunnels", get(api::tunnels::list))
        .route(
            "/tunnels/{id}",
            get(api::tunnels::get_tunnel).delete(api::tunnels::delete),
        )
        .route("/keys", get(api::keys::list).post(api::keys::create))
        .route("/keys/{id}", axum::routing::delete(api::keys::revoke));

    Router::new()
        .nest("/api", api_routes)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
