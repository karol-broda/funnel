use serde::Deserialize;

use super::oauth::{OAuthError, OAuthProvider, OAuthUserInfo};

pub struct GenericProviderConfig {
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub authorize_url: String,
    pub token_url: String,
    pub userinfo_url: String,
    pub scopes: String,
    /// json field name for the user's unique id (default: "sub")
    pub id_field: String,
    /// json field name for email (default: "email")
    pub email_field: String,
    /// json field name for display name (default: "name")
    pub name_field: String,
    /// json field name for avatar url (default: "picture")
    pub avatar_field: String,
}

impl GenericProviderConfig {
    #[cfg(test)]
    pub fn with_defaults(
        name: String,
        client_id: String,
        client_secret: String,
        authorize_url: String,
        token_url: String,
        userinfo_url: String,
        scopes: String,
    ) -> Self {
        Self {
            name,
            client_id,
            client_secret,
            authorize_url,
            token_url,
            userinfo_url,
            scopes,
            id_field: "sub".into(),
            email_field: "email".into(),
            name_field: "name".into(),
            avatar_field: "picture".into(),
        }
    }
}

pub struct GenericProvider {
    config: GenericProviderConfig,
    client: reqwest::Client,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

impl GenericProvider {
    pub fn new(config: GenericProviderConfig) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("funnel-server")
            .build()
            .unwrap_or_default();

        Self { config, client }
    }

    fn extract_string(obj: &serde_json::Value, field: &str) -> Option<String> {
        let val = obj.get(field)?;
        // handle both string and numeric ids
        val.as_str()
            .map(ToString::to_string)
            .or_else(|| val.as_u64().map(|n| n.to_string()))
            .or_else(|| val.as_i64().map(|n| n.to_string()))
    }
}

#[async_trait::async_trait]
impl OAuthProvider for GenericProvider {
    fn name(&self) -> &'static str {
        // leak is fine here, provider names are static for the process lifetime
        Box::leak(self.config.name.clone().into_boxed_str())
    }

    fn authorize_url(&self, redirect_uri: &str, state: &str) -> String {
        format!(
            "{}?client_id={}&redirect_uri={}&state={}&scope={}&response_type=code",
            self.config.authorize_url,
            urlencoding::encode(&self.config.client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(state),
            urlencoding::encode(&self.config.scopes),
        )
    }

    async fn exchange_code(&self, code: &str, redirect_uri: &str) -> Result<String, OAuthError> {
        let resp: TokenResponse = self
            .client
            .post(&self.config.token_url)
            .header("accept", "application/json")
            .form(&[
                ("grant_type", "authorization_code"),
                ("client_id", &self.config.client_id),
                ("client_secret", &self.config.client_secret),
                ("code", code),
                ("redirect_uri", redirect_uri),
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
    }

    async fn fetch_user_info(&self, access_token: &str) -> Result<OAuthUserInfo, OAuthError> {
        let user: serde_json::Value = self
            .client
            .get(&self.config.userinfo_url)
            .bearer_auth(access_token)
            .send()
            .await?
            .json()
            .await?;

        let provider_id = Self::extract_string(&user, &self.config.id_field)
            .ok_or_else(|| OAuthError::MissingField(self.config.id_field.clone()))?;

        let email = Self::extract_string(&user, &self.config.email_field)
            .ok_or_else(|| OAuthError::MissingField(self.config.email_field.clone()))?;

        let name = Self::extract_string(&user, &self.config.name_field);
        let avatar_url = Self::extract_string(&user, &self.config.avatar_field);

        Ok(OAuthUserInfo {
            email,
            name,
            avatar_url,
            provider: self.config.name.clone(),
            provider_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> GenericProviderConfig {
        GenericProviderConfig::with_defaults(
            "test".into(),
            "client123".into(),
            "secret456".into(),
            "https://auth.example.com/authorize".into(),
            "https://auth.example.com/token".into(),
            "https://auth.example.com/userinfo".into(),
            "openid email profile".into(),
        )
    }

    #[test]
    fn authorize_url_includes_all_params() {
        let provider = GenericProvider::new(test_config());
        let url = provider.authorize_url("https://myapp.com/callback", "state123");

        assert!(url.starts_with("https://auth.example.com/authorize?"));
        assert!(url.contains("client_id=client123"));
        assert!(url.contains("state=state123"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("scope=openid"));
    }

    #[test]
    fn authorize_url_encodes_special_chars() {
        let mut config = test_config();
        config.scopes = "openid email profile".into();
        let provider = GenericProvider::new(config);
        let url = provider.authorize_url("https://myapp.com/cb?a=1", "s&t");

        assert!(url.contains("redirect_uri=https%3A%2F%2Fmyapp.com%2Fcb%3Fa%3D1"));
        assert!(url.contains("state=s%26t"));
    }

    #[test]
    fn extract_string_from_string_field() {
        let json = serde_json::json!({"sub": "user-123"});
        assert_eq!(
            GenericProvider::extract_string(&json, "sub"),
            Some("user-123".into())
        );
    }

    #[test]
    fn extract_string_from_numeric_field() {
        let json = serde_json::json!({"id": 42});
        assert_eq!(
            GenericProvider::extract_string(&json, "id"),
            Some("42".into())
        );
    }

    #[test]
    fn extract_string_returns_none_for_missing() {
        let json = serde_json::json!({"foo": "bar"});
        assert!(GenericProvider::extract_string(&json, "missing").is_none());
    }

    #[test]
    fn custom_field_mapping() {
        let mut config = test_config();
        config.id_field = "user_id".into();
        config.email_field = "mail".into();
        config.name_field = "display_name".into();
        config.avatar_field = "avatar".into();

        let json = serde_json::json!({
            "user_id": "u-1",
            "mail": "user@corp.com",
            "display_name": "User One",
            "avatar": "https://img.example.com/1.png"
        });

        assert_eq!(
            GenericProvider::extract_string(&json, &config.id_field),
            Some("u-1".into())
        );
        assert_eq!(
            GenericProvider::extract_string(&json, &config.email_field),
            Some("user@corp.com".into())
        );
        assert_eq!(
            GenericProvider::extract_string(&json, &config.name_field),
            Some("User One".into())
        );
        assert_eq!(
            GenericProvider::extract_string(&json, &config.avatar_field),
            Some("https://img.example.com/1.png".into())
        );
    }
}
