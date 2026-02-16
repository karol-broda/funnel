use std::sync::Arc;

use axum::extract::ws::WebSocket;
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::response::Response;
use serde::Deserialize;

use funnel_core::tunnel::TunnelId;

use crate::app::AppState;
use crate::error::AppError;
use crate::tunnel::connection;

#[derive(Deserialize)]
pub struct ConnectParams {
    id: Option<String>,
}

pub async fn upgrade(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ConnectParams>,
    ws: WebSocketUpgrade,
) -> Result<Response, AppError> {
    let tunnel_id = match params.id {
        Some(raw) => TunnelId::new(raw)?,
        None => TunnelId::generate(),
    };

    if state.tunnels.exists(&tunnel_id) {
        return Err(AppError::TunnelConflict(tunnel_id.to_string()));
    }

    tracing::info!(tunnel_id = %tunnel_id, "websocket upgrade accepted");

    Ok(ws.on_upgrade(move |socket| handle_socket(state, tunnel_id, socket)))
}

async fn handle_socket(state: Arc<AppState>, id: TunnelId, socket: WebSocket) {
    let tunnel = connection::spawn(id.clone(), socket);

    if let Err(_returned) = state.tunnels.insert(id.clone(), Arc::clone(&tunnel)) {
        tracing::warn!(tunnel_id = %id, "tunnel id became occupied during upgrade");
        tunnel.close();
        return;
    }

    tracing::info!(
        tunnel_id = %id,
        total_tunnels = state.tunnels.count(),
        "tunnel connected"
    );

    // wait until the tunnel connection drops
    tunnel.cancelled().await;

    let stats = tunnel.stats();
    state.tunnels.remove(&id);

    tracing::info!(
        tunnel_id = %id,
        bytes_in = stats.bytes_in,
        bytes_out = stats.bytes_out,
        requests = stats.requests,
        uptime_secs = tunnel.connected_at().elapsed().as_secs_f64(),
        remaining_tunnels = state.tunnels.count(),
        "tunnel disconnected"
    );
}
