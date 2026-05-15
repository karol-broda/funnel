use std::sync::Arc;

use axum::extract::{Query, State};
use serde::Deserialize;

use crate::app::AppState;
use crate::auth::{Management, Scoped};
use crate::error::AppError;
use crate::response::Many;
use funnel_core::api::TunnelSession;
use funnel_core::api::envelope::ErrorData;

#[derive(Deserialize, utoipa::IntoParams)]
pub struct ListParams {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    all: bool,
}

const fn default_limit() -> i64 {
    50
}

#[utoipa::path(
    get,
    path = "/sessions",
    operation_id = "list_sessions",
    tag = "Profile",
    security(("bearer" = [])),
    params(ListParams),
    responses(
        (status = 200, description = "Tunnel session history", body = Vec<TunnelSession>),
        (status = 401, description = "Unauthorized", body = ErrorData),
    )
)]
pub async fn list(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
    Query(params): Query<ListParams>,
) -> Result<Many<TunnelSession>, AppError> {
    let sessions = if params.all && auth.is_admin() {
        state.sessions.list_all(params.limit).await?
    } else {
        state
            .sessions
            .list_for_user(auth.user_id, params.limit)
            .await?
    };

    Ok(Many(sessions))
}
