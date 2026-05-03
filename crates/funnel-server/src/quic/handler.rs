use std::sync::Arc;

use funnel_core::protocol::{self, Handshake, HandshakeResponse};
use crate::app::AppState;
use crate::tunnel::connection::ActiveTunnel;

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("connection error: {0}")]
    Connection(#[from] quinn::ConnectionError),

    #[error("frame error: {0}")]
    Frame(#[from] protocol::FrameError),

    #[error("tunnel id conflict: {0}")]
    Conflict(String),
}

pub async fn handle_connection(
    conn: quinn::Connection,
    state: Arc<AppState>,
) -> Result<(), ConnectionError> {
    let (mut send, mut recv) = conn
        .accept_bi()
        .await
        .map_err(ConnectionError::Connection)?;

    let handshake: Handshake = protocol::read_meta(&mut recv).await?;
    let tunnel_id = handshake.tunnel_id;

    let tunnel = Arc::new(ActiveTunnel::new(tunnel_id.clone(), conn.clone()));

    if state
        .tunnels
        .insert(tunnel_id.clone(), Arc::clone(&tunnel))
        .is_err()
    {
        let resp = HandshakeResponse::rejected(format!("tunnel id already in use: {tunnel_id}"));
        let _ = protocol::write_meta(&mut send, &resp).await;
        return Err(ConnectionError::Conflict(tunnel_id.to_string()));
    }

    protocol::write_meta(&mut send, &HandshakeResponse::ok()).await?;

    tracing::info!(tunnel_id = %tunnel_id, "tunnel connected via quic");

    metrics::gauge!("funnel_tunnels_active").increment(1.0);
    metrics::counter!("funnel_tunnels_total").increment(1);

    // wait until the connection closes (control stream EOF or transport error)
    let mut buf = [0u8; 1];
    tokio::select! {
        _ = recv.read(&mut buf) => {},
        _ = conn.closed() => {},
    }

    let stats = tunnel.stats();
    state.tunnels.remove(&tunnel_id);

    let session_secs = tunnel.connected_at().elapsed().as_secs_f64();

    metrics::gauge!("funnel_tunnels_active").decrement(1.0);
    metrics::histogram!("funnel_tunnel_session_duration_seconds").record(session_secs);

    tracing::info!(
        tunnel_id = %tunnel_id,
        bytes_in = stats.bytes_in,
        bytes_out = stats.bytes_out,
        requests = stats.requests,
        session_secs = session_secs,
        remaining_tunnels = state.tunnels.count(),
        "tunnel disconnected"
    );

    Ok(())
}
