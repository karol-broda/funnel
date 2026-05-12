use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use serde::Deserialize;

use crate::app::AppState;
use crate::db::accounts::NewAccount;
use crate::db::api_keys::default_scopes;
use crate::db::users::NewUser;
use funnel_core::protocol::PROTOCOL_VERSION;

#[derive(Deserialize)]
pub struct AuthorizeParams {
    cli_port: u16,
}

#[derive(Deserialize)]
pub struct CallbackParams {
    code: String,
    state: String,
}

fn html_error(status: StatusCode, message: &str) -> Response {
    (
        status,
        Html(format!(
            r"<!doctype html><html><body><h1>Error</h1><p>{message}</p></body></html>",
        )),
    )
        .into_response()
}

pub async fn authorize(
    State(state): State<Arc<AppState>>,
    Path(provider): Path<String>,
    Query(params): Query<AuthorizeParams>,
) -> Response {
    let Some(oauth) = &state.oauth_state else {
        return html_error(StatusCode::NOT_FOUND, "oauth not configured");
    };

    let Some(p) = oauth.providers.get(&provider) else {
        return html_error(
            StatusCode::NOT_FOUND,
            &format!("unknown provider: {provider}"),
        );
    };

    let token = match crate::auth::oauth::OAuthState::generate_state_token() {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "failed to generate state token");
            return html_error(StatusCode::INTERNAL_SERVER_ERROR, "internal error");
        }
    };

    oauth.insert_pending(token.clone(), params.cli_port);

    let redirect_uri = format!(
        "{}/auth/v{PROTOCOL_VERSION}/{}/callback",
        oauth.base_url, provider
    );
    let url = p.authorize_url(&redirect_uri, &token);

    Redirect::temporary(&url).into_response()
}

pub async fn callback(
    State(state): State<Arc<AppState>>,
    Path(provider): Path<String>,
    Query(params): Query<CallbackParams>,
) -> Response {
    let Some(oauth) = &state.oauth_state else {
        return html_error(StatusCode::NOT_FOUND, "oauth not configured");
    };

    let Some(p) = oauth.providers.get(&provider) else {
        return html_error(
            StatusCode::NOT_FOUND,
            &format!("unknown provider: {provider}"),
        );
    };

    let Some(pending) = oauth.take_pending(&params.state) else {
        return html_error(StatusCode::BAD_REQUEST, "invalid or expired state");
    };

    let redirect_uri = format!(
        "{}/auth/v{PROTOCOL_VERSION}/{}/callback",
        oauth.base_url, provider
    );

    let access_token = match p.exchange_code(&params.code, &redirect_uri).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "oauth code exchange failed");
            return html_error(
                StatusCode::BAD_GATEWAY,
                "failed to exchange authorization code",
            );
        }
    };

    let info = match p.fetch_user_info(&access_token).await {
        Ok(i) => i,
        Err(e) => {
            tracing::error!(error = %e, "oauth user info fetch failed");
            return html_error(StatusCode::BAD_GATEWAY, "failed to fetch user info");
        }
    };

    // look up existing account by (provider, provider_account_id)
    let existing_account = match state
        .accounts
        .find_by_provider(&info.provider, &info.provider_id)
        .await
    {
        Ok(a) => a,
        Err(e) => {
            tracing::error!(error = %e, "failed to look up account");
            return html_error(StatusCode::INTERNAL_SERVER_ERROR, "internal error");
        }
    };

    let user = if let Some(account) = existing_account {
        // account exists, update linked user profile
        match state
            .users
            .update_profile(
                account.user_id,
                info.name.as_deref(),
                info.avatar_url.as_deref(),
            )
            .await
        {
            Ok(u) => u,
            Err(e) => {
                tracing::error!(error = %e, "failed to update user profile");
                return html_error(StatusCode::INTERNAL_SERVER_ERROR, "internal error");
            }
        }
    } else {
        // no account for this provider, check if user exists by email
        let existing_user = match state.users.find_by_email(&info.email).await {
            Ok(u) => u,
            Err(e) => {
                tracing::error!(error = %e, "failed to look up user by email");
                return html_error(StatusCode::INTERNAL_SERVER_ERROR, "internal error");
            }
        };

        let user = if let Some(user) = existing_user {
            // user exists from a different provider, link this account
            match state
                .users
                .update_profile(user.id, info.name.as_deref(), info.avatar_url.as_deref())
                .await
            {
                Ok(u) => u,
                Err(e) => {
                    tracing::error!(error = %e, "failed to update user profile");
                    return html_error(StatusCode::INTERNAL_SERVER_ERROR, "internal error");
                }
            }
        } else {
            // completely new user
            match state
                .users
                .create(NewUser {
                    email: info.email,
                    name: info.name,
                    avatar_url: info.avatar_url,
                })
                .await
            {
                Ok(u) => u,
                Err(e) => {
                    tracing::error!(error = %e, "failed to create user");
                    return html_error(StatusCode::INTERNAL_SERVER_ERROR, "internal error");
                }
            }
        };

        if let Err(e) = state
            .accounts
            .create(NewAccount {
                user_id: user.id,
                provider: info.provider,
                provider_account_id: info.provider_id,
                metadata: serde_json::json!({}),
            })
            .await
        {
            tracing::error!(error = %e, "failed to create account link");
            return html_error(StatusCode::INTERNAL_SERVER_ERROR, "internal error");
        }

        // auto promote first user to admin, or match initial admin email
        let should_promote = match state.users.count().await {
            Ok(1) => true,
            Ok(_) => {
                state
                    .initial_admin_email
                    .as_ref()
                    .is_some_and(|email| email == &user.email)
                    && !user.is_admin()
            }
            Err(_) => false,
        };

        if should_promote {
            match state.users.update_role(user.id, "admin").await {
                Ok(promoted) => {
                    tracing::info!(
                        user_id = %promoted.id,
                        email = %promoted.email,
                        "auto promoted user to admin"
                    );
                    promoted
                }
                Err(e) => {
                    tracing::error!(error = %e, "failed to promote user to admin");
                    user
                }
            }
        } else {
            user
        }
    };

    // revoke any existing cli key before issuing a new one
    let _ = state.api_keys.revoke_by_name(user.id, "cli").await;

    let (plaintext, _) = match state
        .api_keys
        .create(user.id, "cli", &default_scopes(), None)
        .await
    {
        Ok(k) => k,
        Err(e) => {
            tracing::error!(error = %e, "failed to create api key");
            return html_error(StatusCode::INTERNAL_SERVER_ERROR, "internal error");
        }
    };

    let cli_redirect = format!(
        "http://127.0.0.1:{}/callback?token={}",
        pending.cli_port, plaintext
    );

    Html(format!(
        r#"<!doctype html>
<html><head><meta http-equiv="refresh" content="0;url={cli_redirect}"></head>
<body><p>login successful, redirecting...</p>
<script>window.location.href="{cli_redirect}";</script></body></html>"#,
    ))
    .into_response()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use axum::Router;
    use axum::routing::get;

    use crate::api;
    use crate::app::AppState;
    use crate::auth::oauth::{OAuthError, OAuthProvider, OAuthState, OAuthUserInfo};
    use crate::store::BoxFuture;
    use crate::store::health::UptimeHealthReporter;
    use crate::store::turso;
    use crate::tunnel::manager::TunnelManager;

    struct MockProvider;

    impl OAuthProvider for MockProvider {
        fn name(&self) -> &'static str {
            "mock"
        }

        fn authorize_url(&self, redirect_uri: &str, state: &str) -> String {
            format!("https://mock.example.com/auth?redirect_uri={redirect_uri}&state={state}")
        }

        fn exchange_code(
            &self,
            code: &str,
            _redirect_uri: &str,
        ) -> BoxFuture<'_, Result<String, OAuthError>> {
            let code = code.to_string();
            Box::pin(async move {
                if code == "valid_code" {
                    Ok("mock_access_token".into())
                } else {
                    Err(OAuthError::Provider("bad code".into()))
                }
            })
        }

        fn fetch_user_info(
            &self,
            _access_token: &str,
        ) -> BoxFuture<'_, Result<OAuthUserInfo, OAuthError>> {
            Box::pin(async {
                Ok(OAuthUserInfo {
                    email: "test@example.com".into(),
                    name: Some("Test User".into()),
                    avatar_url: None,
                    provider: "mock".into(),
                    provider_id: "12345".into(),
                })
            })
        }
    }

    async fn test_state(with_oauth: bool) -> Arc<AppState> {
        let oauth_state = if with_oauth {
            let mut providers: HashMap<String, Arc<dyn OAuthProvider>> = HashMap::new();
            providers.insert("mock".into(), Arc::new(MockProvider));
            Some(Arc::new(OAuthState::new(
                providers,
                "http://localhost:8080".into(),
            )))
        } else {
            None
        };

        let db = turso::open(":memory:")
            .await
            .unwrap_or_else(|e| panic!("open turso: {e}"));

        Arc::new(AppState {
            tunnels: Arc::new(TunnelManager::new()),
            api_keys: Arc::new(turso::api_key_store::TursoApiKeyStore::new(Arc::clone(&db))),
            users: Arc::new(turso::user_store::TursoUserStore::new(Arc::clone(&db))),
            accounts: Arc::new(turso::account_store::TursoAccountStore::new(Arc::clone(
                &db,
            ))),
            sessions: Arc::new(turso::session_recorder::TursoSessionRecorder::new(
                Arc::clone(&db),
            )),
            teams: Arc::new(turso::team_store::TursoTeamStore::new(db)),
            health: Arc::new(UptimeHealthReporter::new()),
            is_tls: false,
            oauth_state,
            initial_admin_email: None,
            quic_port: 4433,
        })
    }

    fn test_router(state: Arc<AppState>) -> Router {
        Router::new()
            .route("/auth/{provider}/authorize", get(super::authorize))
            .route("/auth/{provider}/callback", get(super::callback))
            .route("/api/me", get(api::me::handler))
            .route("/api/keys", get(api::keys::list).post(api::keys::create))
            .with_state(state)
    }

    #[tokio::test]
    async fn authorize_redirects_to_provider() {
        let state = test_state(true).await;
        let server = axum_test::TestServer::new(test_router(state));

        let resp = server
            .get("/auth/mock/authorize")
            .add_query_param("cli_port", 9999)
            .await;

        assert_eq!(resp.status_code(), 307);
        let location = resp.header("location").to_str().unwrap().to_string();
        assert!(location.starts_with("https://mock.example.com/auth?"));
        assert!(location.contains("redirect_uri="));
        assert!(location.contains("state="));
    }

    #[tokio::test]
    async fn authorize_unknown_provider_returns_404() {
        let state = test_state(true).await;
        let server = axum_test::TestServer::new(test_router(state));

        let resp = server
            .get("/auth/nonexistent/authorize")
            .add_query_param("cli_port", 9999)
            .await;

        assert_eq!(resp.status_code(), 404);
        let body = resp.text();
        assert!(body.contains("unknown provider"));
    }

    #[tokio::test]
    async fn authorize_without_oauth_returns_404() {
        let state = test_state(false).await;
        let server = axum_test::TestServer::new(test_router(state));

        let resp = server
            .get("/auth/mock/authorize")
            .add_query_param("cli_port", 9999)
            .await;

        assert_eq!(resp.status_code(), 404);
        let body = resp.text();
        assert!(body.contains("oauth not configured"));
    }

    #[tokio::test]
    async fn callback_with_invalid_state_returns_400() {
        let state = test_state(true).await;
        let server = axum_test::TestServer::new(test_router(state));

        let resp = server
            .get("/auth/mock/callback")
            .add_query_param("code", "valid_code")
            .add_query_param("state", "bogus_state")
            .await;

        assert_eq!(resp.status_code(), 400);
        let body = resp.text();
        assert!(body.contains("invalid or expired state"));
    }

    #[tokio::test]
    async fn callback_with_bad_code_returns_502() {
        let state = test_state(true).await;
        let oauth = state.oauth_state.as_ref().unwrap();
        oauth.insert_pending("test_state_token".into(), 9999);

        let server = axum_test::TestServer::new(test_router(state));

        let resp = server
            .get("/auth/mock/callback")
            .add_query_param("code", "invalid_code")
            .add_query_param("state", "test_state_token")
            .await;

        assert_eq!(resp.status_code(), 502);
    }

    #[tokio::test]
    async fn callback_full_flow_creates_user_and_returns_token() {
        let state = test_state(true).await;
        let oauth = state.oauth_state.as_ref().unwrap();
        oauth.insert_pending("valid_state".into(), 7777);

        let server = axum_test::TestServer::new(test_router(Arc::clone(&state)));

        let resp = server
            .get("/auth/mock/callback")
            .add_query_param("code", "valid_code")
            .add_query_param("state", "valid_state")
            .await;

        assert_eq!(resp.status_code(), 200);
        let body = resp.text();
        assert!(body.contains("login successful"));
        assert!(body.contains("127.0.0.1:7777/callback?token=sk_"));

        // verify account was created
        let accounts = state
            .accounts
            .list_for_user(
                state
                    .users
                    .find_by_email("test@example.com")
                    .await
                    .unwrap()
                    .unwrap()
                    .id,
            )
            .await
            .unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].provider, "mock");
        assert_eq!(accounts[0].provider_account_id, "12345");
    }

    #[tokio::test]
    async fn callback_state_is_consumed() {
        let state = test_state(true).await;
        let oauth = state.oauth_state.as_ref().unwrap();
        oauth.insert_pending("one_time".into(), 7777);

        let server = axum_test::TestServer::new(test_router(state));

        let resp = server
            .get("/auth/mock/callback")
            .add_query_param("code", "valid_code")
            .add_query_param("state", "one_time")
            .await;
        assert_eq!(resp.status_code(), 200);

        let resp = server
            .get("/auth/mock/callback")
            .add_query_param("code", "valid_code")
            .add_query_param("state", "one_time")
            .await;
        assert_eq!(resp.status_code(), 400);
    }

    #[tokio::test]
    async fn callback_second_login_updates_profile() {
        let state = test_state(true).await;
        let oauth = state.oauth_state.as_ref().unwrap();

        // first login
        oauth.insert_pending("first".into(), 7777);
        let server = axum_test::TestServer::new(test_router(Arc::clone(&state)));
        let resp = server
            .get("/auth/mock/callback")
            .add_query_param("code", "valid_code")
            .add_query_param("state", "first")
            .await;
        assert_eq!(resp.status_code(), 200);

        let user = state
            .users
            .find_by_email("test@example.com")
            .await
            .unwrap()
            .unwrap();

        // second login with same provider reuses user
        oauth.insert_pending("second".into(), 7777);
        let resp = server
            .get("/auth/mock/callback")
            .add_query_param("code", "valid_code")
            .add_query_param("state", "second")
            .await;
        assert_eq!(resp.status_code(), 200);

        // still only one account
        let accounts = state.accounts.list_for_user(user.id).await.unwrap();
        assert_eq!(accounts.len(), 1);

        // only one active cli key (old one was revoked)
        let keys = state.api_keys.list_for_user(user.id).await.unwrap();
        assert_eq!(keys.iter().filter(|k| k.name == "cli").count(), 1);
    }

    #[tokio::test]
    async fn me_requires_auth() {
        let state = test_state(false).await;
        let server = axum_test::TestServer::new(test_router(state));

        let resp = server.get("/api/me").await;
        assert_eq!(resp.status_code(), 401);
    }

    #[tokio::test]
    async fn me_returns_user_after_oauth_login() {
        let state = test_state(true).await;
        let oauth = state.oauth_state.as_ref().unwrap();
        oauth.insert_pending("login_state".into(), 7777);

        let server = axum_test::TestServer::new(test_router(Arc::clone(&state)));

        let resp = server
            .get("/auth/mock/callback")
            .add_query_param("code", "valid_code")
            .add_query_param("state", "login_state")
            .await;
        assert_eq!(resp.status_code(), 200);

        let body = resp.text();
        let token_start = body.find("token=sk_").unwrap() + "token=".len();
        let token_end = body[token_start..].find('"').unwrap() + token_start;
        let token = &body[token_start..token_end];

        let resp = server
            .get("/api/me")
            .add_header("authorization", format!("Bearer {token}"))
            .await;
        assert_eq!(resp.status_code(), 200);

        let user: serde_json::Value = resp.json();
        assert_eq!(user["email"], "test@example.com");
        assert_eq!(user["name"], "Test User");
        assert_eq!(user["role"], "admin");
    }
}
