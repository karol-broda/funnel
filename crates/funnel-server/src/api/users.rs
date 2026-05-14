use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::Deserialize;
use uuid::Uuid;

use funnel_core::api::{Role, SetUserRoleRequest, User};

use crate::app::AppState;
use crate::auth::RequireAdmin;
use crate::error::AppError;
use crate::response::{Many, One};
use funnel_core::api::envelope::ErrorData;

#[derive(Deserialize, utoipa::IntoParams)]
pub struct ListParams {
    #[serde(default = "default_limit")]
    limit: i64,
}

const fn default_limit() -> i64 {
    50
}

#[utoipa::path(
    get,
    path = "/users",
    operation_id = "list_users",
    tag = "Users",
    security(("bearer" = [])),
    params(ListParams),
    responses(
        (status = 200, description = "List of all users", body = Vec<User>),
        (status = 401, description = "Unauthorized", body = ErrorData),
        (status = 403, description = "Admin role required", body = ErrorData),
    )
)]
pub async fn list(
    State(state): State<Arc<AppState>>,
    _admin: RequireAdmin,
    Query(params): Query<ListParams>,
) -> Result<Many<User>, AppError> {
    let users = state.users.list_all(params.limit).await?;
    Ok(Many(users))
}

pub type SetRoleRequest = SetUserRoleRequest;

#[utoipa::path(
    put,
    path = "/users/{id}/role",
    operation_id = "set_user_role",
    tag = "Users",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "User ID")),
    request_body = SetRoleRequest,
    responses(
        (status = 200, description = "User role updated", body = User),
        (status = 400, description = "Invalid role or last admin", body = ErrorData),
        (status = 401, description = "Unauthorized", body = ErrorData),
        (status = 403, description = "Admin role required", body = ErrorData),
        (status = 404, description = "User not found", body = ErrorData),
    )
)]
pub async fn set_role(
    State(state): State<Arc<AppState>>,
    _admin: RequireAdmin,
    Path(id): Path<Uuid>,
    Json(req): Json<SetRoleRequest>,
) -> Result<One<User>, AppError> {
    // prevent demotion of the last admin
    if req.role == Role::Member {
        let target = state
            .users
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("user not found".into()))?;

        if target.is_admin() {
            let admin_count = state.users.count_admins().await?;
            if admin_count <= 1 {
                return Err(AppError::BadRequest("cannot demote the last admin".into()));
            }
        }
    }

    let user = state.users.update_role(id, req.role).await?;
    Ok(One(user))
}

#[utoipa::path(
    post,
    path = "/users/{id}/deactivate",
    operation_id = "deactivate_user",
    tag = "Users",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "User ID")),
    responses(
        (status = 200, description = "User deactivated", body = User),
        (status = 400, description = "Cannot deactivate last admin", body = ErrorData),
        (status = 401, description = "Unauthorized", body = ErrorData),
        (status = 403, description = "Admin role required", body = ErrorData),
        (status = 404, description = "User not found", body = ErrorData),
    )
)]
pub async fn deactivate(
    State(state): State<Arc<AppState>>,
    _admin: RequireAdmin,
    Path(id): Path<Uuid>,
) -> Result<One<User>, AppError> {
    let target = state
        .users
        .find_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".into()))?;

    if target.is_admin() {
        let admin_count = state.users.count_admins().await?;
        if admin_count <= 1 {
            return Err(AppError::BadRequest(
                "cannot deactivate the last admin".into(),
            ));
        }
    }

    let user = state.users.deactivate(id).await?;
    Ok(One(user))
}

#[utoipa::path(
    post,
    path = "/users/{id}/reactivate",
    operation_id = "reactivate_user",
    tag = "Users",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "User ID")),
    responses(
        (status = 200, description = "User reactivated", body = User),
        (status = 401, description = "Unauthorized", body = ErrorData),
        (status = 403, description = "Admin role required", body = ErrorData),
    )
)]
pub async fn reactivate(
    State(state): State<Arc<AppState>>,
    _admin: RequireAdmin,
    Path(id): Path<Uuid>,
) -> Result<One<User>, AppError> {
    let user = state.users.reactivate(id).await?;
    Ok(One(user))
}
