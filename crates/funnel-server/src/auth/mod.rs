pub mod generic;
pub mod github;
pub mod oauth;

use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use uuid::Uuid;

use crate::app::AppState;
use crate::error::AppError;

pub struct AuthUser {
    pub user_id: Uuid,
    pub scopes: Vec<String>,
    pub role: String,
}

impl AuthUser {
    fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope)
    }

    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

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

        let user = state
            .users
            .find_by_id(api_key.user_id)
            .await
            .map_err(AppError::Store)?
            .ok_or(AppError::Unauthorized)?;

        if !user.is_active() {
            return Err(AppError::Unauthorized);
        }

        let scopes = api_key
            .scopes
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(ToString::to_string))
                    .collect()
            })
            .unwrap_or_default();

        Ok(Self {
            user_id: api_key.user_id,
            scopes,
            role: user.role,
        })
    }
}

// scope aware extractors using the type system
//
// handlers declare their required scope in the type signature:
//   async fn list(auth: Scoped<Management>) -> ...
//
// the extractor validates auth and checks the scope, returning 403 if missing.

pub trait Scope: Send + Sync + 'static {
    const NAME: &'static str;
}

pub struct Management;
impl Scope for Management {
    const NAME: &'static str = "management";
}

#[allow(dead_code)]
pub struct Tunnels;
impl Scope for Tunnels {
    const NAME: &'static str = "tunnels";
}

pub struct Scoped<S: Scope> {
    pub user_id: Uuid,
    pub role: String,
    _scope: std::marker::PhantomData<S>,
}

impl<S: Scope> Scoped<S> {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

impl<S: Scope> FromRequestParts<Arc<AppState>> for Scoped<S> {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let auth = AuthUser::from_request_parts(parts, state).await?;

        if !auth.has_scope(S::NAME) {
            return Err(AppError::Forbidden);
        }

        Ok(Self {
            user_id: auth.user_id,
            role: auth.role,
            _scope: std::marker::PhantomData,
        })
    }
}

/// extractor that requires management scope AND admin role
pub struct RequireAdmin {
    #[allow(dead_code)]
    pub user_id: Uuid,
}

impl FromRequestParts<Arc<AppState>> for RequireAdmin {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let auth = AuthUser::from_request_parts(parts, state).await?;

        if !auth.has_scope("management") || !auth.is_admin() {
            return Err(AppError::Forbidden);
        }

        Ok(Self {
            user_id: auth.user_id,
        })
    }
}
