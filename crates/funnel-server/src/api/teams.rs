use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use uuid::Uuid;

use funnel_core::api::{
    AddMemberRequest, CreateTeamRequest, SetMemberRoleRequest, Team, TeamMembership, TeamRole,
};

use crate::app::AppState;
use crate::auth::{Management, RequireAdmin, Scoped};
use crate::error::AppError;
use crate::response::{Many, One};
use funnel_core::api::envelope::ErrorData;

async fn can_manage_team(
    state: &AppState,
    team_id: Uuid,
    auth: &Scoped<Management>,
) -> Result<bool, AppError> {
    if auth.is_admin() {
        return Ok(true);
    }
    let membership = state.teams.find_membership(team_id, auth.user_id).await?;
    Ok(membership.is_some_and(|m| m.role == TeamRole::Owner))
}

#[utoipa::path(
    get,
    path = "/teams",
    operation_id = "list_teams",
    tag = "Teams",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "List of teams", body = Vec<Team>),
        (status = 401, description = "Unauthorized", body = ErrorData),
    )
)]
pub async fn list(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
) -> Result<Many<Team>, AppError> {
    let teams = if auth.is_admin() {
        state.teams.list_all().await?
    } else {
        state.teams.list_teams_for_user(auth.user_id).await?
    };
    Ok(Many(teams))
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
        (status = 401, description = "Unauthorized", body = ErrorData),
        (status = 403, description = "Admin role required", body = ErrorData),
        (status = 409, description = "Team name already exists", body = ErrorData),
    )
)]
pub async fn create(
    State(state): State<Arc<AppState>>,
    admin: RequireAdmin,
    Json(req): Json<CreateTeamRequest>,
) -> Result<One<Team>, AppError> {
    let team = state.teams.create(&req.name).await?;
    let owner_id = req.owner_id.unwrap_or(admin.user_id);
    state
        .teams
        .add_member(team.id, owner_id, TeamRole::Owner)
        .await?;
    Ok(One(team))
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
        (status = 401, description = "Unauthorized", body = ErrorData),
        (status = 403, description = "Admin role required", body = ErrorData),
        (status = 404, description = "Team not found", body = ErrorData),
    )
)]
pub async fn delete(
    State(state): State<Arc<AppState>>,
    _admin: RequireAdmin,
    Path(id): Path<Uuid>,
) -> Result<One<serde_json::Value>, AppError> {
    let deleted = state.teams.delete(id).await?;
    if !deleted {
        return Err(AppError::NotFound("team not found".into()));
    }
    Ok(One(serde_json::json!({ "deleted": true })))
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
        (status = 401, description = "Unauthorized", body = ErrorData),
        (status = 403, description = "Not a team member", body = ErrorData),
        (status = 404, description = "Team not found", body = ErrorData),
    )
)]
pub async fn list_members(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
    Path(id): Path<Uuid>,
) -> Result<Many<TeamMembership>, AppError> {
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
    Ok(Many(members))
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
        (status = 401, description = "Unauthorized", body = ErrorData),
        (status = 403, description = "Not a team owner", body = ErrorData),
        (status = 404, description = "Team not found", body = ErrorData),
        (status = 409, description = "Already a member", body = ErrorData),
    )
)]
pub async fn add_member(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
    Path(id): Path<Uuid>,
    Json(req): Json<AddMemberRequest>,
) -> Result<One<TeamMembership>, AppError> {
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
        .add_member(id, req.user_id, TeamRole::Member)
        .await?;
    Ok(One(membership))
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
        (status = 400, description = "Cannot remove last owner", body = ErrorData),
        (status = 401, description = "Unauthorized", body = ErrorData),
        (status = 403, description = "Not a team owner", body = ErrorData),
        (status = 404, description = "Membership not found", body = ErrorData),
    )
)]
pub async fn remove_member(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<One<serde_json::Value>, AppError> {
    if !can_manage_team(&state, id, &auth).await? {
        return Err(AppError::Forbidden);
    }

    // prevent removing the last owner
    let membership = state.teams.find_membership(id, user_id).await?;
    if let Some(ref m) = membership
        && m.role == TeamRole::Owner
    {
        let owner_count = state.teams.count_owners(id).await?;
        if owner_count <= 1 {
            return Err(AppError::BadRequest(
                "cannot remove the last owner of a team".into(),
            ));
        }
    }

    let removed = state.teams.remove_member(id, user_id).await?;
    if !removed {
        return Err(AppError::NotFound("membership not found".into()));
    }
    Ok(One(serde_json::json!({ "removed": true })))
}

pub type SetRoleRequest = SetMemberRoleRequest;

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
        (status = 400, description = "Invalid role or last owner", body = ErrorData),
        (status = 401, description = "Unauthorized", body = ErrorData),
        (status = 403, description = "Not a team owner", body = ErrorData),
        (status = 404, description = "Membership not found", body = ErrorData),
    )
)]
pub async fn set_member_role(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<SetRoleRequest>,
) -> Result<One<TeamMembership>, AppError> {
    if !can_manage_team(&state, id, &auth).await? {
        return Err(AppError::Forbidden);
    }

    // prevent demoting the last owner
    if req.role == TeamRole::Member {
        let current = state
            .teams
            .find_membership(id, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("membership not found".into()))?;
        if current.role == TeamRole::Owner {
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
        .update_member_role(id, user_id, req.role)
        .await?;
    Ok(One(membership))
}
