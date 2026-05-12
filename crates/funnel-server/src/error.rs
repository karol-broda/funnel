use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use funnel_core::tunnel::id::TunnelIdError;
use serde::Serialize;

use crate::store::StoreError;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub(crate) struct ApiErrorBody {
    error: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("tunnel not found: {0}")]
    TunnelNotFound(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("invalid tunnel id: {0}")]
    InvalidTunnelId(#[from] TunnelIdError),

    #[error("store error: {0}")]
    Store(#[from] StoreError),

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden")]
    Forbidden,

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            Self::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            Self::TunnelNotFound(_) | Self::NotFound(_) => {
                (StatusCode::NOT_FOUND, self.to_string())
            }
            Self::InvalidTunnelId(_) | Self::BadRequest(_) => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            Self::Store(e) => match e {
                StoreError::NotFound => (StatusCode::NOT_FOUND, "not found".to_string()),
                StoreError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
                StoreError::Database(db_err) => {
                    tracing::error!(error = %db_err, "database error");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "internal error".to_string(),
                    )
                }
                StoreError::Other(msg) => {
                    tracing::error!(error = %msg, "store error");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "internal error".to_string(),
                    )
                }
            },
            Self::Internal(e) => {
                tracing::error!(error = %e, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal error".to_string(),
                )
            }
        };

        (status, Json(ApiErrorBody { error: message })).into_response()
    }
}
