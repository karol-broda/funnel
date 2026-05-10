use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::Deserialize;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::RequireAdmin;
use crate::db::users::{User, ROLE_ADMIN, ROLE_MEMBER};
use crate::error::AppError;

#[derive(Deserialize)]
pub struct ListParams {
    #[serde(default = "default_limit")]
    limit: i64,
}

const fn default_limit() -> i64 {
    50
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    _admin: RequireAdmin,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<User>>, AppError> {
    let users = state.users.list_all(params.limit).await?;
    Ok(Json(users))
}

#[derive(Deserialize)]
pub struct SetRoleRequest {
    pub role: String,
}

pub async fn set_role(
    State(state): State<Arc<AppState>>,
    _admin: RequireAdmin,
    Path(id): Path<Uuid>,
    Json(req): Json<SetRoleRequest>,
) -> Result<Json<User>, AppError> {
    if req.role != ROLE_ADMIN && req.role != ROLE_MEMBER {
        return Err(AppError::BadRequest(format!(
            "invalid role: {}, must be '{}' or '{}'",
            req.role, ROLE_ADMIN, ROLE_MEMBER
        )));
    }

    // prevent demotion of the last admin
    if req.role == ROLE_MEMBER {
        let target = state
            .users
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("user not found".into()))?;

        if target.is_admin() {
            let admin_count = state.users.count_admins().await?;
            if admin_count <= 1 {
                return Err(AppError::BadRequest(
                    "cannot demote the last admin".into(),
                ));
            }
        }
    }

    let user = state.users.update_role(id, &req.role).await?;
    Ok(Json(user))
}

pub async fn deactivate(
    State(state): State<Arc<AppState>>,
    _admin: RequireAdmin,
    Path(id): Path<Uuid>,
) -> Result<Json<User>, AppError> {
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
    Ok(Json(user))
}

pub async fn reactivate(
    State(state): State<Arc<AppState>>,
    _admin: RequireAdmin,
    Path(id): Path<Uuid>,
) -> Result<Json<User>, AppError> {
    let user = state.users.reactivate(id).await?;
    Ok(Json(user))
}
