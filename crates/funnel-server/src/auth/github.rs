use serde::Deserialize;

use crate::store::BoxFuture;

use super::oauth::{OAuthError, OAuthProvider, OAuthUserInfo};

pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
}

pub struct GitHubProvider {
    config: OAuthConfig,
    client: reqwest::Client,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Deserialize)]
struct GitHubUser {
    id: u64,
    email: Option<String>,
    name: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct GitHubEmail {
    email: String,
    primary: bool,
    verified: bool,
}

impl GitHubProvider {
    pub fn new(config: OAuthConfig) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("funnel-server")
            .build()
            .unwrap_or_default();

        Self { config, client }
    }
}

impl OAuthProvider for GitHubProvider {
    fn name(&self) -> &'static str {
        "github"
    }

    fn authorize_url(&self, redirect_uri: &str, state: &str) -> String {
        format!(
            "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&state={}&scope=user:email",
            urlencoding::encode(&self.config.client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(state),
        )
    }

    fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> BoxFuture<'_, Result<String, OAuthError>> {
        let code = code.to_string();
        let redirect_uri = redirect_uri.to_string();

        Box::pin(async move {
            let resp: TokenResponse = self
                .client
                .post("https://github.com/login/oauth/access_token")
                .header("accept", "application/json")
                .form(&[
                    ("client_id", self.config.client_id.as_str()),
                    ("client_secret", self.config.client_secret.as_str()),
                    ("code", &code),
                    ("redirect_uri", &redirect_uri),
                ])
                .send()
                .await?
                .json()
                .await?;

            if let Some(err) = resp.error {
                let detail = resp.error_description.unwrap_or_default();
                return Err(OAuthError::Provider(format!("{err}: {detail}")));
            }

            resp.access_token
                .ok_or_else(|| OAuthError::MissingField("access_token".into()))
        })
    }

    fn fetch_user_info(
        &self,
        access_token: &str,
    ) -> BoxFuture<'_, Result<OAuthUserInfo, OAuthError>> {
        let token = access_token.to_string();

        Box::pin(async move {
            let user: GitHubUser = self
                .client
                .get("https://api.github.com/user")
                .bearer_auth(&token)
                .send()
                .await?
                .json()
                .await?;

            let email = if let Some(email) = user.email {
                email
            } else {
                // fetch from /user/emails for users with private emails
                let emails: Vec<GitHubEmail> = self
                    .client
                    .get("https://api.github.com/user/emails")
                    .bearer_auth(&token)
                    .send()
                    .await?
                    .json()
                    .await?;

                emails
                    .into_iter()
                    .find(|e| e.primary && e.verified)
                    .map(|e| e.email)
                    .ok_or_else(|| OAuthError::MissingField("email".into()))?
            };

            Ok(OAuthUserInfo {
                email,
                name: user.name,
                avatar_url: user.avatar_url,
                provider: "github".into(),
                provider_id: user.id.to_string(),
            })
        })
    }
}
