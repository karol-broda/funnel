use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use funnel_core::tunnel::id::TunnelIdError;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct ApiErrorBody {
    error: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("tunnel not found: {0}")]
    TunnelNotFound(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("invalid tunnel id: {0}")]
    InvalidTunnelId(#[from] TunnelIdError),

    #[error("database error")]
    Database(#[from] sqlx::Error),

    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Self::TunnelNotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            Self::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            Self::BadRequest(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            Self::InvalidTunnelId(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            Self::Database(e) => {
                tracing::error!(error = %e, "database error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal error".to_string(),
                )
            }
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
