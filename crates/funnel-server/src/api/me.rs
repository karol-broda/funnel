use std::sync::Arc;

use axum::Json;
use axum::extract::State;

use crate::app::AppState;
use crate::auth::{Management, Scoped};
use crate::db::users::User;
use crate::error::AppError;

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
