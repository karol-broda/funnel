use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::{Management, Scoped};
use crate::db::api_keys::{ApiKeyView, default_scopes};
use crate::error::AppError;

#[derive(Deserialize)]
pub struct CreateKeyRequest {
    pub name: String,
    pub scopes: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct CreateKeyResponse {
    pub key: String,
    pub info: ApiKeyView,
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
) -> Result<Json<Vec<ApiKeyView>>, AppError> {
    let keys = state.api_keys.list_for_user(auth.user_id).await?;
    Ok(Json(keys))
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
    Json(req): Json<CreateKeyRequest>,
) -> Result<Json<CreateKeyResponse>, AppError> {
    let scopes = req.scopes.map_or_else(default_scopes, |s| {
        serde_json::Value::Array(s.into_iter().map(serde_json::Value::String).collect())
    });

    let (plaintext, info) = state
        .api_keys
        .create(auth.user_id, &req.name, &scopes)
        .await?;

    Ok(Json(CreateKeyResponse {
        key: plaintext,
        info,
    }))
}

pub async fn revoke(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let revoked = state.api_keys.revoke(id, auth.user_id).await?;
    if !revoked {
        return Err(AppError::NotFound("api key not found".into()));
    }

    Ok(Json(serde_json::json!({ "revoked": true })))
}
