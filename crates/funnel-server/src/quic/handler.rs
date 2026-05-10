use std::sync::Arc;

use crate::app::AppState;
use crate::tunnel::connection::ActiveTunnel;
use funnel_core::protocol::PROTOCOL_VERSION;
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

    match handshake.version {
        Some(v) if v != PROTOCOL_VERSION => {
            let resp = HandshakeResponse::rejected(format!(
                "incompatible client version: got v{v}, server requires v{PROTOCOL_VERSION}"
            ));
            let _ = frame::write_meta(&mut send, &resp).await;
            return Err(ConnectionError::Auth(format!(
                "incompatible client version: v{v}"
            )));
        }
        None => {
            let resp = HandshakeResponse::rejected("client too old, version field required");
            let _ = frame::write_meta(&mut send, &resp).await;
            return Err(ConnectionError::Auth(
                "client did not send version".into(),
            ));
        }
        _ => {}
    }

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

    // check user is active
    let user = state
        .users
        .find_by_id(user_id)
        .await
        .map_err(|e| ConnectionError::Auth(format!("user lookup failed: {e}")))?
        .ok_or_else(|| ConnectionError::Auth("user not found".into()))?;

    if !user.is_active() {
        let resp = HandshakeResponse::rejected("user account is deactivated");
        let _ = frame::write_meta(&mut send, &resp).await;
        return Err(ConnectionError::Auth("deactivated user".into()));
    }

    // resolve team if specified
    let team_id = if let Some(ref team_name) = handshake.team {
        let team = state
            .teams
            .find_by_name(team_name)
            .await
            .map_err(|e| ConnectionError::Auth(format!("team lookup failed: {e}")))?
            .ok_or_else(|| {
                ConnectionError::Auth(format!("team not found: {team_name}"))
            })?;

        let is_member = state
            .teams
            .is_member(team.id, user_id)
            .await
            .map_err(|e| ConnectionError::Auth(format!("membership check failed: {e}")))?;

        if !is_member {
            let resp = HandshakeResponse::rejected(format!(
                "not a member of team: {team_name}"
            ));
            let _ = frame::write_meta(&mut send, &resp).await;
            return Err(ConnectionError::Auth(format!(
                "not a member of team: {team_name}"
            )));
        }

        Some(team.id)
    } else {
        None
    };

    let tunnel = Arc::new(ActiveTunnel::new(
        tunnel_id.clone(),
        conn.clone(),
        user_id,
        team_id,
    ));

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

    tracing::info!(
        tunnel_id = %tunnel_id,
        user_id = %user_id,
        team_id = ?team_id,
        "tunnel connected via quic"
    );

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
