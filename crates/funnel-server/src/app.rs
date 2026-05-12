use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::get;
use metrics_exporter_prometheus::PrometheusHandle;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use scalar_api_reference::axum::scalar_response;

use funnel_core::protocol::PROTOCOL_VERSION;
use funnel_core::tunnel::id::TunnelId;

use crate::api;
use crate::auth::oauth::OAuthState;
use crate::metrics;
use crate::proxy;
use crate::store::account_store::AccountStore;
use crate::store::api_key_store::ApiKeyStore;
use crate::store::health::HealthReporter;
use crate::store::session_recorder::SessionRecorder;
use crate::store::team_store::TeamStore;
use crate::store::tunnel_registry::TunnelRegistry;
use crate::store::user_store::UserStore;

pub struct AppState {
    pub tunnels: Arc<dyn TunnelRegistry>,
    pub api_keys: Arc<dyn ApiKeyStore>,
    pub users: Arc<dyn UserStore>,
    pub accounts: Arc<dyn AccountStore>,
    pub sessions: Arc<dyn SessionRecorder>,
    pub teams: Arc<dyn TeamStore>,
    pub health: Arc<dyn HealthReporter>,
    pub is_tls: bool,
    pub oauth_state: Option<Arc<OAuthState>>,
    pub initial_admin_email: Option<String>,
    pub quic_port: u16,
}

pub fn build_router(state: Arc<AppState>, metrics_handle: PrometheusHandle) -> Router {
    let api_routes = Router::new()
        .route("/health", get(api::health::handler))
        .route("/info", get(api::info::handler))
        .route("/tunnels", get(api::tunnels::list))
        .route(
            "/tunnels/{id}",
            get(api::tunnels::get_tunnel).delete(api::tunnels::delete),
        )
        .route("/keys", get(api::keys::list).post(api::keys::create))
        .route("/keys/{id}", axum::routing::delete(api::keys::revoke))
        .route("/me", get(api::me::handler))
        .route("/accounts", get(api::accounts::list))
        .route("/sessions", get(api::sessions::list))
        .route(
            "/users",
            get(api::users::list),
        )
        .route("/users/{id}/role", axum::routing::put(api::users::set_role))
        .route(
            "/users/{id}/deactivate",
            axum::routing::post(api::users::deactivate),
        )
        .route(
            "/users/{id}/reactivate",
            axum::routing::post(api::users::reactivate),
        )
        .route(
            "/teams",
            get(api::teams::list).post(api::teams::create),
        )
        .route("/teams/{id}", axum::routing::delete(api::teams::delete))
        .route(
            "/teams/{id}/members",
            get(api::teams::list_members).post(api::teams::add_member),
        )
        .route(
            "/teams/{id}/members/{user_id}",
            axum::routing::delete(api::teams::remove_member),
        )
        .route(
            "/teams/{id}/members/{user_id}/role",
            axum::routing::put(api::teams::set_member_role),
        )
        .route("/metrics", get(metrics::handler).with_state(metrics_handle));

    let auth_routes = Router::new()
        .route("/{provider}/authorize", get(api::oauth::authorize))
        .route("/{provider}/callback", get(api::oauth::callback));

    let api_prefix = format!("/api/v{PROTOCOL_VERSION}");
    let auth_prefix = format!("/auth/v{PROTOCOL_VERSION}");

    let scalar_config = serde_json::json!({
        "url": format!("{api_prefix}/openapi.json"),
        "hideClientButton": true,
        "agent": { "disabled": true },
        "mcp": { "disabled": true },
    });

    let scalar_routes = Router::new()
        .route(
            "/scalar",
            get(move || {
                let config = scalar_config.clone();
                async move { scalar_response(&config, None) }
            }),
        )
        .route("/openapi.json", get(crate::openapi::json_handler));

    let api_routes = api_routes.merge(scalar_routes);

    Router::new()
        .nest(&api_prefix, api_routes)
        .nest(&auth_prefix, auth_routes)
        .fallback(proxy::router::handle_tunnel_request)
        .layer(middleware::from_fn_with_state(
            Arc::clone(&state),
            tunnel_routing,
        ))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}

/// if the first label of the host header matches a registered tunnel,
/// route directly to the tunnel proxy, bypassing api/auth routes
async fn tunnel_routing(
    State(state): State<Arc<AppState>>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let tunnel_id = request
        .headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .and_then(proxy::router::extract_subdomain)
        .and_then(|sub| TunnelId::new(sub).ok())
        .filter(|id| state.tunnels.get(id).is_some());

    if let Some(id) = tunnel_id {
        request.extensions_mut().insert(id);
        proxy::router::handle_tunnel_request(State(state), request).await
    } else {
        next.run(request).await
    }
}
