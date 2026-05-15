use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use uuid::Uuid;

use funnel_core::api::{ApiKeyView, CreateKeyRequest, CreateKeyResponse};

use crate::app::AppState;
use crate::auth::{Management, Scoped};
use crate::db::api_keys::default_scopes;
use crate::error::AppError;
use crate::openapi::TagSeo;
use crate::response::{Many, One};
use funnel_core::api::envelope::ErrorData;

pub const TAG_SEO: TagSeo = TagSeo {
    tag: "API Keys",
    title: "API keys: create, list, and revoke scoped access tokens",
    description: "REST API for managing funnel API keys with scoped permissions. \
                  Create keys for CI/CD pipelines, list existing keys, and revoke compromised tokens.",
    keywords: &[
        "API key management",
        "scoped access tokens",
        "create API key",
        "revoke API key",
        "CI/CD tunnel authentication",
    ],
};

#[utoipa::path(
    get,
    path = "/keys",
    operation_id = "list_keys",
    tag = "API Keys",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "List of API keys for the current user", body = Vec<ApiKeyView>),
        (status = 401, description = "Unauthorized", body = ErrorData),
    )
)]
pub async fn list(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
) -> Result<Many<ApiKeyView>, AppError> {
    let keys = state.api_keys.list_for_user(auth.user_id).await?;
    Ok(Many(keys))
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
        (status = 401, description = "Unauthorized", body = ErrorData),
    )
)]
pub async fn create(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
    Json(req): Json<CreateKeyRequest>,
) -> Result<One<CreateKeyResponse>, AppError> {
    let scopes = req.scopes.unwrap_or_else(default_scopes);

    let (plaintext, info) = state
        .api_keys
        .create(auth.user_id, &req.name, &scopes, req.expires_at)
        .await?;

    Ok(One(CreateKeyResponse {
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
        (status = 401, description = "Unauthorized", body = ErrorData),
        (status = 404, description = "Key not found", body = ErrorData),
    )
)]
pub async fn revoke(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
    Path(id): Path<Uuid>,
) -> Result<One<serde_json::Value>, AppError> {
    let revoked = state.api_keys.revoke(id, auth.user_id).await?;
    if !revoked {
        return Err(AppError::NotFound("api key not found".into()));
    }

    Ok(One(serde_json::json!({ "revoked": true })))
}
