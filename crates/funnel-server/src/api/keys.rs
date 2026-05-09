use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::AuthUser;
use crate::db::api_keys::ApiKeyView;
use crate::error::AppError;

#[derive(Deserialize)]
pub struct CreateKeyRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct CreateKeyResponse {
    pub key: String,
    pub info: ApiKeyView,
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    AuthUser(user_id): AuthUser,
) -> Result<Json<Vec<ApiKeyView>>, AppError> {
    let keys = state.api_keys.list_for_user(user_id).await?;
    Ok(Json(keys))
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<CreateKeyRequest>,
) -> Result<Json<CreateKeyResponse>, AppError> {
    let (plaintext, info) = state.api_keys.create(user_id, &req.name).await?;

    Ok(Json(CreateKeyResponse {
        key: plaintext,
        info,
    }))
}

pub async fn revoke(
    State(state): State<Arc<AppState>>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let revoked = state.api_keys.revoke(id, user_id).await?;
    if !revoked {
        return Err(AppError::NotFound("api key not found".into()));
    }

    Ok(Json(serde_json::json!({ "revoked": true })))
}
