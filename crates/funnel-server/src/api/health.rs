use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use serde::Serialize;

use crate::app::AppState;

#[derive(Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    pub status: &'static str,
    pub uptime_secs: u64,
}

#[utoipa::path(
    get,
    path = "/health",
    operation_id = "get_health",
    tag = "Server",
    responses(
        (status = 200, description = "Server is healthy", body = HealthResponse),
    )
)]
pub async fn handler(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: state.health.status(),
        uptime_secs: state.health.uptime_secs(),
    })
}
