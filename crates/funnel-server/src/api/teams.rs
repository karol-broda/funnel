use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use serde::Deserialize;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::{Management, RequireAdmin, Scoped};
use crate::db::teams::{Team, TeamMembership, TEAM_ROLE_MEMBER, TEAM_ROLE_OWNER};
use crate::error::{ApiErrorBody, AppError};

async fn can_manage_team(state: &AppState, team_id: Uuid, auth: &Scoped<Management>) -> Result<bool, AppError> {
    if auth.is_admin() {
        return Ok(true);
    }
    let membership = state.teams.find_membership(team_id, auth.user_id).await?;
    Ok(membership.map_or(false, |m| m.role == TEAM_ROLE_OWNER))
}

#[utoipa::path(
    get,
    path = "/teams",
    operation_id = "list_teams",
    tag = "Teams",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "List of teams", body = Vec<Team>),
        (status = 401, description = "Unauthorized", body = ApiErrorBody),
    )
)]
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

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateTeamRequest {
    pub name: String,
    pub owner_id: Option<Uuid>,
}

#[utoipa::path(
    post,
    path = "/teams",
    operation_id = "create_team",
    tag = "Teams",
    security(("bearer" = [])),
    request_body = CreateTeamRequest,
    responses(
        (status = 200, description = "Team created", body = Team),
        (status = 401, description = "Unauthorized", body = ApiErrorBody),
        (status = 403, description = "Admin role required", body = ApiErrorBody),
        (status = 409, description = "Team name already exists", body = ApiErrorBody),
    )
)]
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

#[utoipa::path(
    delete,
    path = "/teams/{id}",
    operation_id = "delete_team",
    tag = "Teams",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Team ID")),
    responses(
        (status = 200, description = "Team deleted", body = Object),
        (status = 401, description = "Unauthorized", body = ApiErrorBody),
        (status = 403, description = "Admin role required", body = ApiErrorBody),
        (status = 404, description = "Team not found", body = ApiErrorBody),
    )
)]
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

#[utoipa::path(
    get,
    path = "/teams/{id}/members",
    operation_id = "list_team_members",
    tag = "Teams",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Team ID")),
    responses(
        (status = 200, description = "List of team members", body = Vec<TeamMembership>),
        (status = 401, description = "Unauthorized", body = ApiErrorBody),
        (status = 403, description = "Not a team member", body = ApiErrorBody),
        (status = 404, description = "Team not found", body = ApiErrorBody),
    )
)]
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

#[derive(Deserialize, utoipa::ToSchema)]
pub struct AddMemberRequest {
    pub user_id: Uuid,
}

#[utoipa::path(
    post,
    path = "/teams/{id}/members",
    operation_id = "add_team_member",
    tag = "Teams",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "Team ID")),
    request_body = AddMemberRequest,
    responses(
        (status = 200, description = "Member added", body = TeamMembership),
        (status = 401, description = "Unauthorized", body = ApiErrorBody),
        (status = 403, description = "Not a team owner", body = ApiErrorBody),
        (status = 404, description = "Team not found", body = ApiErrorBody),
        (status = 409, description = "Already a member", body = ApiErrorBody),
    )
)]
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

#[utoipa::path(
    delete,
    path = "/teams/{id}/members/{user_id}",
    operation_id = "remove_team_member",
    tag = "Teams",
    security(("bearer" = [])),
    params(
        ("id" = Uuid, Path, description = "Team ID"),
        ("user_id" = Uuid, Path, description = "User ID"),
    ),
    responses(
        (status = 200, description = "Member removed", body = Object),
        (status = 400, description = "Cannot remove last owner", body = ApiErrorBody),
        (status = 401, description = "Unauthorized", body = ApiErrorBody),
        (status = 403, description = "Not a team owner", body = ApiErrorBody),
        (status = 404, description = "Membership not found", body = ApiErrorBody),
    )
)]
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

#[derive(Deserialize, utoipa::ToSchema)]
#[schema(as = SetTeamMemberRoleRequest)]
pub struct SetRoleRequest {
    pub role: String,
}

#[utoipa::path(
    put,
    path = "/teams/{id}/members/{user_id}/role",
    operation_id = "set_team_member_role",
    tag = "Teams",
    security(("bearer" = [])),
    params(
        ("id" = Uuid, Path, description = "Team ID"),
        ("user_id" = Uuid, Path, description = "User ID"),
    ),
    request_body = SetRoleRequest,
    responses(
        (status = 200, description = "Member role updated", body = TeamMembership),
        (status = 400, description = "Invalid role or last owner", body = ApiErrorBody),
        (status = 401, description = "Unauthorized", body = ApiErrorBody),
        (status = 403, description = "Not a team owner", body = ApiErrorBody),
        (status = 404, description = "Membership not found", body = ApiErrorBody),
    )
)]
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
