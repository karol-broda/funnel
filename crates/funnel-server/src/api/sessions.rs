use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;

use crate::app::AppState;
use crate::auth::{Management, Scoped};
use crate::db::tunnel_sessions::TunnelSession;
use crate::error::AppError;

#[derive(Deserialize)]
pub struct ListParams {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    all: bool,
}

const fn default_limit() -> i64 {
    50
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<TunnelSession>>, AppError> {
    let sessions = if params.all && auth.is_admin() {
        state.sessions.list_all(params.limit).await?
    } else {
        state
            .sessions
            .list_for_user(auth.user_id, params.limit)
            .await?
    };

    Ok(Json(sessions))
}
