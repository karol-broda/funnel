use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;

use crate::store::BoxFuture;

#[derive(Debug, Clone)]
pub struct OAuthUserInfo {
    pub email: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub provider: String,
    pub provider_id: String,
}

#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    #[error("http request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("provider returned an error: {0}")]
    Provider(String),

    #[error("missing required field: {0}")]
    MissingField(String),
}

#[allow(dead_code)]
pub trait OAuthProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn authorize_url(&self, redirect_uri: &str, state: &str) -> String;
    fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> BoxFuture<'_, Result<String, OAuthError>>;
    fn fetch_user_info(
        &self,
        access_token: &str,
    ) -> BoxFuture<'_, Result<OAuthUserInfo, OAuthError>>;
}

pub struct PendingAuth {
    pub cli_port: u16,
    pub created_at: Instant,
}

const STATE_TOKEN_BYTES: usize = 32;
const PENDING_TTL_SECS: u64 = 600;

pub struct OAuthState {
    pub providers: HashMap<String, Arc<dyn OAuthProvider>>,
    pub pending: DashMap<String, PendingAuth>,
    pub base_url: String,
}

impl OAuthState {
    pub fn new(providers: HashMap<String, Arc<dyn OAuthProvider>>, base_url: String) -> Self {
        Self {
            providers,
            pending: DashMap::new(),
            base_url,
        }
    }

    pub fn generate_state_token() -> Result<String, getrandom::Error> {
        let mut bytes = [0u8; STATE_TOKEN_BYTES];
        getrandom::fill(&mut bytes)?;
        Ok(hex::encode(bytes))
    }

    pub fn insert_pending(&self, state: String, cli_port: u16) {
        self.cleanup_expired();
        self.pending.insert(
            state,
            PendingAuth {
                cli_port,
                created_at: Instant::now(),
            },
        );
    }

    pub fn take_pending(&self, state: &str) -> Option<PendingAuth> {
        self.pending.remove(state).map(|(_, v)| v)
    }

    fn cleanup_expired(&self) {
        self.pending
            .retain(|_, v| v.created_at.elapsed().as_secs() < PENDING_TTL_SECS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_state() -> OAuthState {
        OAuthState::new(HashMap::new(), "http://localhost".into())
    }

    #[test]
    fn generate_state_token_produces_64_hex_chars() {
        let token = OAuthState::generate_state_token().unwrap();
        assert_eq!(token.len(), STATE_TOKEN_BYTES * 2);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn generated_tokens_are_unique() {
        let a = OAuthState::generate_state_token().unwrap();
        let b = OAuthState::generate_state_token().unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn insert_and_take_pending() {
        let state = empty_state();

        state.insert_pending("tok_1".into(), 9000);

        let pending = state.take_pending("tok_1");
        assert!(pending.is_some());
        assert_eq!(pending.unwrap().cli_port, 9000);
    }

    #[test]
    fn take_pending_removes_entry() {
        let state = empty_state();

        state.insert_pending("tok_2".into(), 9001);
        let _ = state.take_pending("tok_2");

        assert!(state.take_pending("tok_2").is_none());
    }

    #[test]
    fn take_pending_returns_none_for_unknown() {
        let state = empty_state();
        assert!(state.take_pending("nonexistent").is_none());
    }

    #[test]
    fn expired_entries_are_cleaned_up() {
        let state = empty_state();

        // insert an already expired entry by manipulating directly
        state.pending.insert(
            "old".into(),
            PendingAuth {
                cli_port: 1234,
                created_at: Instant::now() - std::time::Duration::from_secs(PENDING_TTL_SECS + 1),
            },
        );

        // inserting a new one triggers cleanup
        state.insert_pending("fresh".into(), 5678);

        assert!(state.take_pending("old").is_none());
        assert!(state.take_pending("fresh").is_some());
    }
}
