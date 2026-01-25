use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use serde::Serialize;

use crate::app::AppState;
use crate::error::AppError;

#[derive(Serialize)]
pub struct TunnelInfo {
    pub id: String,
    pub status: &'static str,
}

/// List all active tunnels.
pub async fn list(State(_state): State<Arc<AppState>>) -> Json<Vec<TunnelInfo>> {
    // TODO: wire to TunnelManager once tunnel module is implemented
    Json(vec![])
}

/// Get a specific tunnel by ID.
pub async fn get_tunnel(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<TunnelInfo>, AppError> {
    // TODO: wire to TunnelManager
    Err(AppError::TunnelNotFound(id))
}

/// Delete (force-close) a tunnel.
pub async fn delete(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: wire to TunnelManager
    Err(AppError::TunnelNotFound(id))
}
