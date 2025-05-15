use std::sync::Arc;

use crate::app::AppState;
use crate::tunnel::connection::ActiveTunnel;
use funnel_core::api::ApiScope;
use funnel_core::protocol::PROTOCOL_VERSION;
use funnel_core::protocol::error_codes::AppCode;
use funnel_core::protocol::frame;
use funnel_core::protocol::handshake::{
    Handshake, HandshakeResult, RoutingMode, ServerLimits, TunnelResult, TunnelSpec, TunnelType,
};
use funnel_core::tunnel::id::TunnelId;

fn server_limits(state: &AppState) -> ServerLimits {
    let mut types = vec![TunnelType::Http];
    if state.tcp_tunnels_enabled {
        types.push(TunnelType::Stream);
    }
    ServerLimits::default().with_tunnel_types(types)
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("connection error: {0}")]
    Connection(#[from] quinn::ConnectionError),

    #[error("frame error: {0}")]
    Frame(#[from] frame::FrameError),

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

    if handshake.version != PROTOCOL_VERSION {
        let result = HandshakeResult {
            version: PROTOCOL_VERSION,
            server_id: state.server_id.clone(),
            tunnels: vec![],
            limits: server_limits(&state),
        };
        let _ = frame::write_meta(&mut send, &result).await;
        return Err(ConnectionError::Auth(format!(
            "incompatible client version: got v{}, server is v{PROTOCOL_VERSION}",
            handshake.version
        )));
    }

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

    if !api_key.has_scope(ApiScope::Tunnels) {
        let result = HandshakeResult {
            version: PROTOCOL_VERSION,
            server_id: state.server_id.clone(),
            tunnels: handshake
                .tunnels
                .iter()
                .map(|t| {
                    TunnelResult::error(
                        t.id.clone(),
                        AppCode::ScopeInsufficient,
                        "token missing tunnels scope",
                    )
                })
                .collect(),
            limits: server_limits(&state),
        };
        let _ = frame::write_meta(&mut send, &result).await;
        return Err(ConnectionError::Auth("missing tunnels scope".into()));
    }

    let user_id = api_key.user_id;

    let user = state
        .users
        .find_by_id(user_id)
        .await
        .map_err(|e| ConnectionError::Auth(format!("user lookup failed: {e}")))?
        .ok_or_else(|| ConnectionError::Auth("user not found".into()))?;

    if !user.is_active() {
        let result = HandshakeResult {
            version: PROTOCOL_VERSION,
            server_id: state.server_id.clone(),
            tunnels: handshake
                .tunnels
                .iter()
                .map(|t| {
                    TunnelResult::error(
                        t.id.clone(),
                        AppCode::UserDeactivated,
                        "user account is deactivated",
                    )
                })
                .collect(),
            limits: server_limits(&state),
        };
        let _ = frame::write_meta(&mut send, &result).await;
        return Err(ConnectionError::Auth("deactivated user".into()));
    }

    let mut tunnel_results = Vec::new();
    let mut registered_ids: Vec<TunnelId> = Vec::new();

    for spec in &handshake.tunnels {
        let result = register_tunnel(spec, user_id, &conn, &state).await;
        if result.is_ok() {
            registered_ids.push(spec.id.clone());
        }
        tunnel_results.push(result);
    }

    let handshake_result = HandshakeResult {
        version: PROTOCOL_VERSION,
        server_id: state.server_id.clone(),
        tunnels: tunnel_results,
        limits: ServerLimits::default(),
    };

    frame::write_meta(&mut send, &handshake_result).await?;

    let accepted_count = registered_ids.len();

    if accepted_count == 0 {
        return Ok(());
    }

    tracing::info!(
        tunnels = ?registered_ids.iter().map(ToString::to_string).collect::<Vec<_>>(),
        user_id = %user_id,
        "tunnels connected via quic"
    );

    metrics::gauge!("funnel_tunnels_active").increment(accepted_count as f64);
    metrics::counter!("funnel_tunnels_total").increment(accepted_count as u64);

    let client_ip = conn.remote_address();
    let ip_network: Option<ipnetwork::IpNetwork> = Some(client_ip.ip().into());

    let mut sessions = Vec::new();
    for tunnel_id in &registered_ids {
        let session = state
            .sessions
            .record_connect(user_id, tunnel_id.as_ref(), ip_network)
            .await
            .ok();
        sessions.push((tunnel_id.clone(), session));
    }

    let mut buf = [0u8; 1];
    tokio::select! {
        _ = recv.read(&mut buf) => {},
        _ = conn.closed() => {},
    }

    for (tunnel_id, session) in &sessions {
        let stats = state
            .tunnels
            .get(tunnel_id)
            .map(|t| (t.stats(), t.connected_at().elapsed()));

        state.stream_listeners.stop(tunnel_id);
        state.tunnels.remove(tunnel_id);

        if let (Some((stats, duration)), Some(session)) = (stats, session) {
            metrics::histogram!("funnel_tunnel_session_duration_seconds")
                .record(duration.as_secs_f64());

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
    }

    metrics::gauge!("funnel_tunnels_active").decrement(accepted_count as f64);

    tracing::info!(
        tunnels = ?registered_ids.iter().map(ToString::to_string).collect::<Vec<_>>(),
        user_id = %user_id,
        remaining_tunnels = state.tunnels.count(),
        "tunnels disconnected"
    );

    Ok(())
}

async fn register_tunnel(
    spec: &TunnelSpec,
    user_id: uuid::Uuid,
    conn: &quinn::Connection,
    state: &AppState,
) -> TunnelResult {
    match spec.tunnel_type {
        TunnelType::Http => {}
        TunnelType::Stream => {
            if !state.tcp_tunnels_enabled {
                return TunnelResult::error(
                    spec.id.clone(),
                    AppCode::UnsupportedTunnelType,
                    "tcp tunnels are not enabled on this server",
                );
            }
        }
        TunnelType::Dgram => {
            return TunnelResult::error(
                spec.id.clone(),
                AppCode::UnsupportedTunnelType,
                format!("unsupported tunnel type: {}", spec.tunnel_type),
            );
        }
    }

    if spec.tunnel_type == TunnelType::Stream && spec.routing == Some(RoutingMode::Sni) {
        return TunnelResult::error(
            spec.id.clone(),
            AppCode::UnsupportedTunnelType,
            "sni routing is not yet supported",
        );
    }

    let team_id = match resolve_team(&spec.id, spec.team.as_deref(), user_id, state).await {
        Ok(id) => id,
        Err(result) => return result,
    };

    // for stream tunnels: bind the TCP port first so we know the allocated port
    // before creating the tunnel. the listener is started after registration.
    let tcp_listener = if spec.tunnel_type == TunnelType::Stream {
        match state
            .stream_listeners
            .bind(spec.remote_port, &state.host)
            .await
        {
            Ok(pair) => Some(pair),
            Err(e) => {
                return TunnelResult::error(
                    spec.id.clone(),
                    AppCode::PortUnavailable,
                    e.to_string(),
                );
            }
        }
    } else {
        None
    };

    let remote_port = tcp_listener.as_ref().map(|(_, port)| *port);

    let tunnel = Arc::new(ActiveTunnel::new(
        spec.id.clone(),
        conn.clone(),
        spec.tunnel_type.clone(),
        remote_port,
        user_id,
        team_id,
    ));

    if state
        .tunnels
        .insert(spec.id.clone(), Arc::clone(&tunnel))
        .is_err()
    {
        return TunnelResult::error(
            spec.id.clone(),
            AppCode::TunnelIdConflict,
            format!("tunnel id already in use: {}", spec.id),
        );
    }

    // start the TCP listener now that the tunnel is registered
    if let Some((listener, port)) = tcp_listener {
        state
            .stream_listeners
            .run(spec.id.clone(), tunnel, listener, port);
    }

    match spec.tunnel_type {
        TunnelType::Http => TunnelResult::ok(spec.id.clone()),
        TunnelType::Stream => TunnelResult::ok_with_port(spec.id.clone(), remote_port.unwrap_or(0)),
        TunnelType::Dgram => unreachable!(),
    }
}

async fn resolve_team(
    tunnel_id: &TunnelId,
    team_name: Option<&str>,
    user_id: uuid::Uuid,
    state: &AppState,
) -> Result<Option<uuid::Uuid>, TunnelResult> {
    let Some(team_name) = team_name else {
        return Ok(None);
    };

    let team = state
        .teams
        .find_by_name(team_name)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "team lookup failed");
            TunnelResult::error(
                tunnel_id.clone(),
                AppCode::TeamNotFound,
                format!("team lookup failed: {e}"),
            )
        })?
        .ok_or_else(|| {
            TunnelResult::error(
                tunnel_id.clone(),
                AppCode::TeamNotFound,
                format!("team not found: {team_name}"),
            )
        })?;

    let is_member = state.teams.is_member(team.id, user_id).await.map_err(|e| {
        TunnelResult::error(
            tunnel_id.clone(),
            AppCode::TeamMembershipRequired,
            format!("membership check failed: {e}"),
        )
    })?;

    if !is_member {
        return Err(TunnelResult::error(
            tunnel_id.clone(),
            AppCode::TeamMembershipRequired,
            format!("not a member of team: {team_name}"),
        ));
    }

    Ok(Some(team.id))
}
