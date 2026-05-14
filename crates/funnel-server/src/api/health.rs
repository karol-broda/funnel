use std::sync::Arc;

use axum::extract::State;

use funnel_core::api::HealthResponse;

use crate::app::AppState;
use crate::response::One;

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
