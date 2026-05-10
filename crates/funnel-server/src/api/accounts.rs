use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use serde::Serialize;
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::{Management, Scoped};
use crate::error::AppError;

#[derive(Serialize)]
pub struct AccountView {
    pub id: Uuid,
    pub provider: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
) -> Result<Json<Vec<AccountView>>, AppError> {
    let accounts = state.accounts.list_for_user(auth.user_id).await?;

    let views: Vec<AccountView> = accounts
        .into_iter()
        .map(|a| AccountView {
            id: a.id,
            provider: a.provider,
            created_at: a.created_at,
        })
        .collect();

    Ok(Json(views))
}
