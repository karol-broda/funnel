use std::sync::Arc;

use axum::Router;
use axum::routing::get;
use metrics_exporter_prometheus::PrometheusHandle;
use sqlx::PgPool;
use tokio::time::Instant;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::api;
use crate::error::AppError;
use crate::metrics;
use crate::proxy;
use crate::tunnel::manager::TunnelManager;

pub struct AppState {
    pub db: Option<PgPool>,
    pub tunnels: TunnelManager,
    pub start_time: Instant,
    pub is_tls: bool,
}

impl AppState {
    pub fn new(db: Option<PgPool>, is_tls: bool) -> Self {
        Self {
            db,
            tunnels: TunnelManager::new(),
            start_time: Instant::now(),
            is_tls,
        }
    }

    pub fn require_db(&self) -> Result<&PgPool, AppError> {
        self.db
            .as_ref()
            .ok_or_else(|| AppError::BadRequest("database not configured".into()))
    }
}

pub fn build_router(state: Arc<AppState>, metrics_handle: PrometheusHandle) -> Router {
    let api_routes = Router::new()
        .route("/health", get(api::health::handler))
        .route("/tunnels", get(api::tunnels::list))
        .route(
            "/tunnels/{id}",
            get(api::tunnels::get_tunnel).delete(api::tunnels::delete),
        )
        .route("/keys", get(api::keys::list).post(api::keys::create))
        .route("/keys/{id}", axum::routing::delete(api::keys::revoke))
        .route("/metrics", get(metrics::handler).with_state(metrics_handle));

    Router::new()
        .nest("/api", api_routes)
        .fallback(proxy::router::handle_tunnel_request)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
