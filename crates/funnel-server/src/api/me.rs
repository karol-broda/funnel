use std::sync::Arc;

use axum::Json;
use axum::extract::State;

use crate::app::AppState;
use crate::auth::{Management, Scoped};
use crate::db::users::User;
use crate::error::{ApiErrorBody, AppError};

#[utoipa::path(
    get,
    path = "/me",
    operation_id = "get_current_user",
    tag = "Profile",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Current user profile", body = User),
        (status = 401, description = "Unauthorized", body = ApiErrorBody),
        (status = 404, description = "User not found", body = ApiErrorBody),
    )
)]
pub async fn handler(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
) -> Result<Json<User>, AppError> {
    state
        .users
        .find_by_id(auth.user_id)
        .await?
        .ok_or(AppError::NotFound("user not found".into()))
        .map(Json)
}
