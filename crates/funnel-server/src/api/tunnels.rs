use std::sync::Arc;

use axum::extract::{Path, State};

use funnel_core::tunnel::id::TunnelId;

use crate::app::AppState;
use crate::auth::{Management, Scoped};
use crate::error::AppError;
use crate::response::{Many, One};
use funnel_core::api::TunnelInfo;
use funnel_core::api::envelope::ErrorData;

#[utoipa::path(
    get,
    path = "/tunnels",
    operation_id = "list_tunnels",
    tag = "Tunnels",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "List of active tunnels", body = Vec<TunnelInfo>),
        (status = 401, description = "Unauthorized", body = ErrorData),
        (status = 403, description = "Forbidden", body = ErrorData),
    )
)]
pub async fn list(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
) -> Result<Many<TunnelInfo>, AppError> {
    let all = state.tunnels.list();

    let tunnels = if auth.is_admin() {
        all
    } else {
        let team_ids = state
            .teams
            .get_team_ids_for_user(auth.user_id)
            .await
            .map_err(AppError::Store)?;

        all.into_iter()
            .filter(|t| {
                t.owner_id == auth.user_id || t.team_id.is_some_and(|tid| team_ids.contains(&tid))
            })
            .collect()
    };

    Ok(Many(tunnels))
}

fn can_access(tunnel: &TunnelInfo, user_id: uuid::Uuid, team_ids: &[uuid::Uuid]) -> bool {
    tunnel.owner_id == user_id || tunnel.team_id.is_some_and(|tid| team_ids.contains(&tid))
}

#[utoipa::path(
    get,
    path = "/tunnels/{id}",
    operation_id = "get_tunnel",
    tag = "Tunnels",
    security(("bearer" = [])),
    params(("id" = String, Path, description = "Tunnel ID")),
    responses(
        (status = 200, description = "Tunnel details", body = TunnelInfo),
        (status = 401, description = "Unauthorized", body = ErrorData),
        (status = 403, description = "Forbidden", body = ErrorData),
        (status = 404, description = "Tunnel not found", body = ErrorData),
    )
)]
pub async fn get_tunnel(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
    Path(id): Path<String>,
) -> Result<One<TunnelInfo>, AppError> {
    let tunnel_id = TunnelId::new(id.clone())?;

    let tunnel = state
        .tunnels
        .get(&tunnel_id)
        .ok_or_else(|| AppError::TunnelNotFound(id))?;

    let info = TunnelInfo {
        id: tunnel.id().to_string(),
        uptime_secs: tunnel.connected_at().elapsed().as_secs_f64(),
        stats: tunnel.stats(),
        owner_id: tunnel.owner_id(),
        team_id: tunnel.team_id(),
    };

    if !auth.is_admin() {
        let team_ids = state
            .teams
            .get_team_ids_for_user(auth.user_id)
            .await
            .map_err(AppError::Store)?;
        if !can_access(&info, auth.user_id, &team_ids) {
            return Err(AppError::Forbidden);
        }
    }

    Ok(One(info))
}

#[utoipa::path(
    delete,
    path = "/tunnels/{id}",
    operation_id = "delete_tunnel",
    tag = "Tunnels",
    security(("bearer" = [])),
    params(("id" = String, Path, description = "Tunnel ID")),
    responses(
        (status = 200, description = "Tunnel disconnected", body = Object),
        (status = 401, description = "Unauthorized", body = ErrorData),
        (status = 403, description = "Forbidden", body = ErrorData),
        (status = 404, description = "Tunnel not found", body = ErrorData),
    )
)]
pub async fn delete(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
    Path(id): Path<String>,
) -> Result<One<serde_json::Value>, AppError> {
    let tunnel_id = TunnelId::new(id.clone())?;

    let tunnel = state
        .tunnels
        .get(&tunnel_id)
        .ok_or_else(|| AppError::TunnelNotFound(id))?;

    if !auth.is_admin() {
        let info = TunnelInfo {
            id: tunnel.id().to_string(),
            uptime_secs: 0.0,
            stats: tunnel.stats(),
            owner_id: tunnel.owner_id(),
            team_id: tunnel.team_id(),
        };
        let team_ids = state
            .teams
            .get_team_ids_for_user(auth.user_id)
            .await
            .map_err(AppError::Store)?;
        if !can_access(&info, auth.user_id, &team_ids) {
            return Err(AppError::Forbidden);
        }
    }

    // drop the Arc from get() before removing
    drop(tunnel);

    let tunnel = state
        .tunnels
        .remove(&tunnel_id)
        .ok_or_else(|| AppError::TunnelNotFound(tunnel_id.to_string()))?;

    tunnel.close();

    Ok(One(serde_json::json!({ "deleted": true })))
}
