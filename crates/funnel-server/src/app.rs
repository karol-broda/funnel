use std::sync::Arc;

use axum::Router;
use axum::routing::get;
use metrics_exporter_prometheus::PrometheusHandle;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::api;
use crate::metrics;
use crate::proxy;
use crate::store::api_key_store::ApiKeyStore;
use crate::store::health::HealthReporter;
use crate::store::session_recorder::SessionRecorder;
use crate::store::tunnel_registry::TunnelRegistry;
use crate::store::user_store::UserStore;

pub struct AppState {
    pub tunnels: Arc<dyn TunnelRegistry>,
    pub api_keys: Arc<dyn ApiKeyStore>,
    #[allow(dead_code)]
    pub users: Arc<dyn UserStore>,
    #[allow(dead_code)]
    pub sessions: Arc<dyn SessionRecorder>,
    pub health: Arc<dyn HealthReporter>,
    pub is_tls: bool,
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
