use std::sync::Arc;

use axum::extract::State;

use funnel_core::api::{ServerCapabilities, ServerInfo};
use funnel_core::protocol::PROTOCOL_VERSION;

use crate::app::AppState;
use crate::response::One;

#[utoipa::path(
    get,
    path = "/info",
    operation_id = "get_server_info",
    tag = "Server",
    responses(
        (status = 200, description = "Server info, capabilities, and configuration", body = ServerInfo),
    )
)]
pub async fn handler(State(state): State<Arc<AppState>>) -> One<ServerInfo> {
    let mut tunnel_types = vec!["http".to_string()];
    if state.tcp_tunnels_enabled {
        tunnel_types.push("tcp".to_string());
    }

    let oauth_providers = state
        .oauth_state
        .as_ref()
        .map(|o| o.provider_names())
        .unwrap_or_default();

    let info = ServerInfo {
        version: PROTOCOL_VERSION,
        quic_port: state.quic_port,
        capabilities: ServerCapabilities {
            tunnel_types,
            tls: state.is_tls,
            oauth_providers,
        },
    };
    One(info)
}
