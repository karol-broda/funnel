use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};

use funnel_core::tunnel::id::TunnelId;

use crate::app::AppState;
use crate::error::AppError;
use crate::tunnel::manager::TunnelInfo;

pub async fn list(State(state): State<Arc<AppState>>) -> Json<Vec<TunnelInfo>> {
    Json(state.tunnels.list())
}

pub async fn get_tunnel(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<TunnelInfo>, AppError> {
    let tunnel_id = TunnelId::new(id.clone())?;

    let tunnel = state
        .tunnels
        .get(&tunnel_id)
        .ok_or_else(|| AppError::TunnelNotFound(id))?;

    Ok(Json(TunnelInfo {
        id: tunnel.id().to_string(),
        uptime_secs: tunnel.connected_at().elapsed().as_secs_f64(),
        stats: tunnel.stats(),
    }))
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let tunnel_id = TunnelId::new(id.clone())?;

    let tunnel = state
        .tunnels
        .remove(&tunnel_id)
        .ok_or_else(|| AppError::TunnelNotFound(id))?;

    tunnel.close();

    Ok(Json(serde_json::json!({ "deleted": true })))
}
