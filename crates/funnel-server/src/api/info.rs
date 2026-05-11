use std::sync::Arc;

use axum::Json;
use axum::extract::State;

use funnel_core::api::ServerInfo;
use funnel_core::protocol::PROTOCOL_VERSION;

use crate::app::AppState;

pub async fn handler(State(state): State<Arc<AppState>>) -> Json<ServerInfo> {
    Json(ServerInfo {
        version: PROTOCOL_VERSION,
        quic_port: state.quic_port,
    })
}
