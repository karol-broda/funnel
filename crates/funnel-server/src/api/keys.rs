use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::AppState;
use crate::auth::{Management, Scoped};
use crate::db::api_keys::{ApiKeyView, default_scopes};
use crate::error::{ApiErrorBody, AppError};

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateKeyRequest {
    pub name: String,
    pub scopes: Option<Vec<String>>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct CreateKeyResponse {
    pub key: String,
    pub info: ApiKeyView,
}

#[utoipa::path(
    get,
    path = "/keys",
    operation_id = "list_keys",
    tag = "API Keys",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "List of API keys for the current user", body = Vec<ApiKeyView>),
        (status = 401, description = "Unauthorized", body = ApiErrorBody),
    )
)]
pub async fn list(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
) -> Result<Json<Vec<ApiKeyView>>, AppError> {
    let keys = state.api_keys.list_for_user(auth.user_id).await?;
    Ok(Json(keys))
}

#[utoipa::path(
    post,
    path = "/keys",
    operation_id = "create_key",
    tag = "API Keys",
    security(("bearer" = [])),
    request_body = CreateKeyRequest,
    responses(
        (status = 200, description = "API key created", body = CreateKeyResponse),
        (status = 401, description = "Unauthorized", body = ApiErrorBody),
    )
)]
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
        .create(auth.user_id, &req.name, &scopes, req.expires_at)
        .await?;

    Ok(Json(CreateKeyResponse {
        key: plaintext,
        info,
    }))
}

#[utoipa::path(
    delete,
    path = "/keys/{id}",
    operation_id = "revoke_key",
    tag = "API Keys",
    security(("bearer" = [])),
    params(("id" = Uuid, Path, description = "API key ID")),
    responses(
        (status = 200, description = "API key revoked", body = Object),
        (status = 401, description = "Unauthorized", body = ApiErrorBody),
        (status = 404, description = "Key not found", body = ApiErrorBody),
    )
)]
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
