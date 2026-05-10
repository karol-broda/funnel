use std::sync::Arc;

use crate::app::AppState;
use crate::tunnel::connection::ActiveTunnel;
use funnel_core::protocol::frame;
use funnel_core::protocol::handshake::{Handshake, HandshakeResponse};

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("connection error: {0}")]
    Connection(#[from] quinn::ConnectionError),

    #[error("frame error: {0}")]
    Frame(#[from] frame::FrameError),

    #[error("tunnel id conflict: {0}")]
    Conflict(String),

    #[error("auth error: {0}")]
    Auth(String),
}

pub async fn handle_connection(
    conn: quinn::Connection,
    state: Arc<AppState>,
) -> Result<(), ConnectionError> {
    let (mut send, mut recv) = conn
        .accept_bi()
        .await
        .map_err(ConnectionError::Connection)?;

    let handshake: Handshake = frame::read_meta(&mut recv).await?;
    let tunnel_id = handshake.tunnel_id;

    // validate auth token
    let token = handshake
        .token
        .as_deref()
        .ok_or_else(|| ConnectionError::Auth("missing token".into()))?;

    let api_key = state
        .api_keys
        .validate(token)
        .await
        .map_err(|e| ConnectionError::Auth(format!("validation failed: {e}")))?
        .ok_or_else(|| ConnectionError::Auth("invalid token".into()))?;

    if !api_key.has_scope("tunnels") {
        let resp = HandshakeResponse::rejected("token missing tunnels scope");
        let _ = frame::write_meta(&mut send, &resp).await;
        return Err(ConnectionError::Auth("missing tunnels scope".into()));
    }

    let user_id = api_key.user_id;

    let tunnel = Arc::new(ActiveTunnel::new(tunnel_id.clone(), conn.clone()));

    if state
        .tunnels
        .insert(tunnel_id.clone(), Arc::clone(&tunnel))
        .is_err()
    {
        let resp = HandshakeResponse::rejected(format!("tunnel id already in use: {tunnel_id}"));
        let _ = frame::write_meta(&mut send, &resp).await;
        return Err(ConnectionError::Conflict(tunnel_id.to_string()));
    }

    frame::write_meta(&mut send, &HandshakeResponse::ok()).await?;

    tracing::info!(tunnel_id = %tunnel_id, user_id = %user_id, "tunnel connected via quic");

    metrics::gauge!("funnel_tunnels_active").increment(1.0);
    metrics::counter!("funnel_tunnels_total").increment(1);

    let client_ip = conn.remote_address();
    let ip_network: Option<ipnetwork::IpNetwork> = Some(client_ip.ip().into());

    let session = state
        .sessions
        .record_connect(user_id, tunnel_id.as_ref(), ip_network)
        .await
        .ok();

    // wait until the connection closes (control stream EOF or transport error)
    let mut buf = [0u8; 1];
    tokio::select! {
        _ = recv.read(&mut buf) => {},
        _ = conn.closed() => {},
    }

    let stats = tunnel.stats();
    state.tunnels.remove(&tunnel_id);

    if let Some(session) = session {
        let _ = state
            .sessions
            .record_disconnect(
                session.id,
                i64::try_from(stats.bytes_in).unwrap_or(i64::MAX),
                i64::try_from(stats.bytes_out).unwrap_or(i64::MAX),
                i64::try_from(stats.requests).unwrap_or(i64::MAX),
            )
            .await;
    }

    let session_secs = tunnel.connected_at().elapsed().as_secs_f64();

    metrics::gauge!("funnel_tunnels_active").decrement(1.0);
    metrics::histogram!("funnel_tunnel_session_duration_seconds").record(session_secs);

    tracing::info!(
        tunnel_id = %tunnel_id,
        user_id = %user_id,
        bytes_in = stats.bytes_in,
        bytes_out = stats.bytes_out,
        requests = stats.requests,
        session_secs = session_secs,
        remaining_tunnels = state.tunnels.count(),
        "tunnel disconnected"
    );

    Ok(())
}
