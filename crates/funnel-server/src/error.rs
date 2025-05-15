use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use funnel_core::api::envelope::Envelope;
use funnel_core::protocol::error_codes::AppCode;
use funnel_core::tunnel::id::TunnelIdError;

use crate::store::StoreError;

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
        let (status, error_code, title) = match &self {
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                AppCode::AuthRequired,
                "unauthorized",
            ),
            Self::Forbidden => (
                StatusCode::FORBIDDEN,
                AppCode::ScopeInsufficient,
                "forbidden",
            ),
            Self::TunnelNotFound(_) => {
                (StatusCode::NOT_FOUND, AppCode::NotFound, "tunnel not found")
            }
            Self::NotFound(_) => (StatusCode::NOT_FOUND, AppCode::NotFound, "not found"),
            Self::InvalidTunnelId(_) => (
                StatusCode::BAD_REQUEST,
                AppCode::TunnelIdInvalid,
                "invalid tunnel id",
            ),
            Self::BadRequest(_) => (StatusCode::BAD_REQUEST, AppCode::BadRequest, "bad request"),
            Self::Store(e) => match e {
                StoreError::NotFound => (StatusCode::NOT_FOUND, AppCode::NotFound, "not found"),
                StoreError::Conflict(_) => {
                    (StatusCode::CONFLICT, AppCode::TunnelIdConflict, "conflict")
                }
                StoreError::Database(db_err) => {
                    tracing::error!(error = %db_err, "database error");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        AppCode::InternalError,
                        "internal error",
                    )
                }
                StoreError::Other(msg) => {
                    tracing::error!(error = %msg, "store error");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        AppCode::InternalError,
                        "internal error",
                    )
                }
            },
            Self::Internal(e) => {
                tracing::error!(error = %e, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    AppCode::InternalError,
                    "internal error",
                )
            }
        };

        let detail = match &self {
            Self::TunnelNotFound(id) => Some(format!("tunnel not found: {id}")),
            Self::NotFound(msg)
            | Self::BadRequest(msg)
            | Self::Store(StoreError::Conflict(msg)) => Some(msg.clone()),
            Self::InvalidTunnelId(e) => Some(e.to_string()),
            _ => None,
        };

        let envelope = Envelope::error(error_code, title, detail, status.as_u16());

        (status, Json(envelope)).into_response()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http_body_util::BodyExt;

    async fn response_json(error: AppError) -> (StatusCode, serde_json::Value) {
        let resp = error.into_response();
        let status = resp.status();
        let body = resp.into_body();
        let bytes = Body::new(body).collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        (status, json)
    }

    #[tokio::test]
    async fn unauthorized_returns_401_with_auth_required() {
        let (status, json) = response_json(AppError::Unauthorized).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(json["kind"], "error");
        assert_eq!(json["data"]["type"], "auth_required");
        assert_eq!(json["data"]["status"], 401);
    }

    #[tokio::test]
    async fn forbidden_returns_403_with_scope_insufficient() {
        let (status, json) = response_json(AppError::Forbidden).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(json["data"]["type"], "scope_insufficient");
        assert_eq!(json["data"]["status"], 403);
    }

    #[tokio::test]
    async fn tunnel_not_found_returns_404_with_detail() {
        let (status, json) = response_json(AppError::TunnelNotFound("abc".into())).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(json["data"]["type"], "not_found");
        assert_eq!(json["data"]["detail"], "tunnel not found: abc");
    }

    #[tokio::test]
    async fn not_found_returns_404_with_detail() {
        let (status, json) = response_json(AppError::NotFound("user not found".into())).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(json["data"]["type"], "not_found");
        assert_eq!(json["data"]["detail"], "user not found");
    }

    #[tokio::test]
    async fn bad_request_returns_400_with_bad_request_code() {
        let (status, json) = response_json(AppError::BadRequest("missing field".into())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json["data"]["type"], "bad_request");
        assert_eq!(json["data"]["detail"], "missing field");
    }

    #[tokio::test]
    async fn invalid_tunnel_id_returns_400() {
        let err = AppError::InvalidTunnelId(TunnelIdError::Empty);
        let (status, json) = response_json(err).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json["data"]["type"], "tunnel_id_invalid");
        assert!(json["data"]["detail"].as_str().is_some());
    }

    #[tokio::test]
    async fn store_not_found_returns_404() {
        let (status, json) = response_json(AppError::Store(StoreError::NotFound)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(json["data"]["type"], "not_found");
    }

    #[tokio::test]
    async fn store_conflict_returns_409_with_detail() {
        let err = AppError::Store(StoreError::Conflict("unique violation".into()));
        let (status, json) = response_json(err).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(json["data"]["type"], "tunnel_id_conflict");
        assert_eq!(json["data"]["detail"], "unique violation");
    }

    #[tokio::test]
    async fn store_other_returns_500() {
        let err = AppError::Store(StoreError::Other("disk full".into()));
        let (status, json) = response_json(err).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(json["data"]["type"], "internal_error");
        // internal errors must not leak details to clients
        assert!(json["data"].get("detail").is_none());
    }

    #[tokio::test]
    async fn internal_error_returns_500_without_detail() {
        let err = AppError::Internal(anyhow::anyhow!("something broke"));
        let (status, json) = response_json(err).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(json["data"]["type"], "internal_error");
        assert!(json["data"].get("detail").is_none());
    }

    #[tokio::test]
    async fn unauthorized_has_no_detail() {
        let (_, json) = response_json(AppError::Unauthorized).await;
        assert!(json["data"].get("detail").is_none());
    }

    #[tokio::test]
    async fn forbidden_has_no_detail() {
        let (_, json) = response_json(AppError::Forbidden).await;
        assert!(json["data"].get("detail").is_none());
    }

    #[tokio::test]
    async fn all_responses_have_error_kind() {
        let errors: Vec<AppError> = vec![
            AppError::Unauthorized,
            AppError::Forbidden,
            AppError::TunnelNotFound("x".into()),
            AppError::NotFound("x".into()),
            AppError::BadRequest("x".into()),
            AppError::Store(StoreError::NotFound),
            AppError::Store(StoreError::Conflict("x".into())),
            AppError::Store(StoreError::Other("x".into())),
            AppError::Internal(anyhow::anyhow!("x")),
        ];

        for err in errors {
            let (_, json) = response_json(err).await;
            assert_eq!(json["kind"], "error", "missing kind=error in response");
            assert!(json["data"]["type"].is_string(), "missing data.type");
            assert!(json["data"]["title"].is_string(), "missing data.title");
            assert!(json["data"]["status"].is_number(), "missing data.status");
        }
    }
}
