use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use serde::Deserialize;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::{Management, RequireAdmin, Scoped};
use crate::db::teams::{Team, TeamMembership, TEAM_ROLE_MEMBER, TEAM_ROLE_OWNER};
use crate::error::AppError;

async fn can_manage_team(state: &AppState, team_id: Uuid, auth: &Scoped<Management>) -> Result<bool, AppError> {
    if auth.is_admin() {
        return Ok(true);
    }
    let membership = state.teams.find_membership(team_id, auth.user_id).await?;
    Ok(membership.map_or(false, |m| m.role == TEAM_ROLE_OWNER))
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
) -> Result<Json<Vec<Team>>, AppError> {
    let teams = if auth.is_admin() {
        state.teams.list_all().await?
    } else {
        state.teams.list_teams_for_user(auth.user_id).await?
    };
    Ok(Json(teams))
}

#[derive(Deserialize)]
pub struct CreateTeamRequest {
    pub name: String,
    pub owner_id: Option<Uuid>,
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    admin: RequireAdmin,
    Json(req): Json<CreateTeamRequest>,
) -> Result<Json<Team>, AppError> {
    let team = state.teams.create(&req.name).await?;
    let owner_id = req.owner_id.unwrap_or(admin.user_id);
    state
        .teams
        .add_member(team.id, owner_id, TEAM_ROLE_OWNER)
        .await?;
    Ok(Json(team))
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    _admin: RequireAdmin,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let deleted = state.teams.delete(id).await?;
    if !deleted {
        return Err(AppError::NotFound("team not found".into()));
    }
    Ok(Json(serde_json::json!({ "deleted": true })))
}

pub async fn list_members(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<TeamMembership>>, AppError> {
    state
        .teams
        .find_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("team not found".into()))?;

    if !auth.is_admin() {
        let is_member = state.teams.is_member(id, auth.user_id).await?;
        if !is_member {
            return Err(AppError::Forbidden);
        }
    }

    let members = state.teams.list_members(id).await?;
    Ok(Json(members))
}

#[derive(Deserialize)]
pub struct AddMemberRequest {
    pub user_id: Uuid,
}

pub async fn add_member(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
    Path(id): Path<Uuid>,
    Json(req): Json<AddMemberRequest>,
) -> Result<Json<TeamMembership>, AppError> {
    state
        .teams
        .find_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("team not found".into()))?;

    if !can_manage_team(&state, id, &auth).await? {
        return Err(AppError::Forbidden);
    }

    let membership = state
        .teams
        .add_member(id, req.user_id, TEAM_ROLE_MEMBER)
        .await?;
    Ok(Json(membership))
}

pub async fn remove_member(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !can_manage_team(&state, id, &auth).await? {
        return Err(AppError::Forbidden);
    }

    // prevent removing the last owner
    let membership = state.teams.find_membership(id, user_id).await?;
    if let Some(ref m) = membership {
        if m.role == TEAM_ROLE_OWNER {
            let owner_count = state.teams.count_owners(id).await?;
            if owner_count <= 1 {
                return Err(AppError::BadRequest(
                    "cannot remove the last owner of a team".into(),
                ));
            }
        }
    }

    let removed = state.teams.remove_member(id, user_id).await?;
    if !removed {
        return Err(AppError::NotFound("membership not found".into()));
    }
    Ok(Json(serde_json::json!({ "removed": true })))
}

#[derive(Deserialize)]
pub struct SetRoleRequest {
    pub role: String,
}

pub async fn set_member_role(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<SetRoleRequest>,
) -> Result<Json<TeamMembership>, AppError> {
    if req.role != TEAM_ROLE_OWNER && req.role != TEAM_ROLE_MEMBER {
        return Err(AppError::BadRequest(format!(
            "invalid role '{}', must be '{}' or '{}'",
            req.role, TEAM_ROLE_OWNER, TEAM_ROLE_MEMBER
        )));
    }

    if !can_manage_team(&state, id, &auth).await? {
        return Err(AppError::Forbidden);
    }

    // prevent demoting the last owner
    if req.role == TEAM_ROLE_MEMBER {
        let current = state
            .teams
            .find_membership(id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("membership not found".into()))?;
        if current.role == TEAM_ROLE_OWNER {
            let owner_count = state.teams.count_owners(id).await?;
            if owner_count <= 1 {
                return Err(AppError::BadRequest(
                    "cannot demote the last owner of a team".into(),
                ));
            }
        }
    }

    let membership = state
        .teams
        .update_member_role(id, user_id, &req.role)
        .await?;
    Ok(Json(membership))
}
