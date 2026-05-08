use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::AppState;
use crate::db::api_keys::ApiKeyView;
use crate::error::AppError;

#[derive(Deserialize)]
pub struct CreateKeyRequest {
    pub name: String,
    // TODO: extract user_id from auth middleware instead
    pub user_id: Uuid,
}

#[derive(Serialize)]
pub struct CreateKeyResponse {
    pub key: String,
    pub info: ApiKeyView,
}

/// List all active API keys for a user.
pub async fn list(State(state): State<Arc<AppState>>) -> Result<Json<Vec<ApiKeyView>>, AppError> {
    // TODO: get user_id from auth middleware
    let _keys = state.api_keys.list_for_user(Uuid::nil()).await?;
    Ok(Json(vec![]))
}

/// Create a new API key.
pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateKeyRequest>,
) -> Result<Json<CreateKeyResponse>, AppError> {
    let (plaintext, info) = state.api_keys.create(req.user_id, &req.name).await?;

    Ok(Json(CreateKeyResponse {
        key: plaintext,
        info,
    }))
}

/// Revoke an API key.
pub async fn revoke(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: get user_id from auth middleware
    let user_id = Uuid::nil();

    let revoked = state.api_keys.revoke(id, user_id).await?;
    if !revoked {
        return Err(AppError::NotFound("api key not found".into()));
    }

    Ok(Json(serde_json::json!({ "revoked": true })))
}
