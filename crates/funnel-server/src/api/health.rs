use std::sync::Arc;

use axum::extract::State;

use funnel_core::api::HealthResponse;

use crate::app::AppState;
use crate::openapi::TagSeo;
use crate::response::One;

pub const TAG_SEO: TagSeo = TagSeo {
    tag: "Server",
    title: "Server endpoints: health check, version, and server info",
    description: "REST API endpoints for monitoring funnel server health status \
                  and retrieving server version and configuration details.",
    keywords: &[
        "funnel health check",
        "server status API",
        "server info endpoint",
        "self-hosted tunnel monitoring",
    ],
};

#[utoipa::path(
    get,
    path = "/health",
    operation_id = "get_health",
    tag = "Server",
    responses(
        (status = 200, description = "Server is healthy", body = HealthResponse),
    )
)]
pub async fn handler(State(state): State<Arc<AppState>>) -> One<HealthResponse> {
    let resp = HealthResponse {
        status: state.health.status().to_string(),
        uptime_secs: state.health.uptime_secs(),
    };
    One(resp)
}
