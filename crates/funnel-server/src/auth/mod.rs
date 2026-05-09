use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use uuid::Uuid;

use crate::app::AppState;
use crate::error::AppError;

pub struct AuthUser(pub Uuid);

impl FromRequestParts<Arc<AppState>> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(AppError::Unauthorized)?;

        let api_key = state
            .api_keys
            .validate(token)
            .await
            .map_err(AppError::Store)?
            .ok_or(AppError::Unauthorized)?;

        Ok(AuthUser(api_key.user_id))
    }
}
