use std::sync::Arc;

use axum::extract::State;

use crate::app::AppState;
use crate::auth::{Management, Scoped};
use funnel_core::api::User;
use crate::error::AppError;
use crate::response::One;
use funnel_core::api::envelope::ErrorData;

#[utoipa::path(
    get,
    path = "/me",
    operation_id = "get_current_user",
    tag = "Profile",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Current user profile", body = User),
        (status = 401, description = "Unauthorized", body = ErrorData),
        (status = 404, description = "User not found", body = ErrorData),
    )
)]
pub async fn handler(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
) -> Result<One<User>, AppError> {
    let user = state
        .users
        .find_by_id(auth.user_id)
        .await?
        .ok_or(AppError::NotFound("user not found".into()))?;
    Ok(One(user))
}
