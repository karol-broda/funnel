use std::fmt;

use base64::prelude::*;
use serde::{Deserialize, Serialize};

const API_KEY_PREFIX: &str = "sk_";
const API_KEY_RANDOM_BYTES: usize = 32;
const DISPLAY_PREFIX_LEN: usize = 10;

/// The visible prefix of an API key (e.g. `"sk_abc123..."`), safe for display and logging.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiKeyPrefix(String);

impl ApiKeyPrefix {
    pub fn from_key(full_key: &str) -> Self {
        let end = full_key.len().min(DISPLAY_PREFIX_LEN);
        Self(full_key[..end].to_string())
    }
}

impl AsRef<str> for ApiKeyPrefix {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ApiKeyPrefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}...", self.0)
    }
}

/// Generate a cryptographically random API key with the `sk_` prefix.
///
/// Returns the full plaintext key. This should be shown to the user exactly once
/// and then only stored as a hash.
pub fn generate_api_key() -> String {
    let mut bytes = [0u8; API_KEY_RANDOM_BYTES];
    getrandom::fill(&mut bytes).expect("failed to generate random bytes");
    format!("{}{}", API_KEY_PREFIX, BASE64_URL_SAFE_NO_PAD.encode(bytes))
}

/// Hash a plaintext API key using SHA-256.
///
/// This is used for storage and constant-time comparison. We use SHA-256 rather
/// than argon2 for API keys because they are high-entropy random tokens, not
/// user-chosen passwords, so brute-force resistance from a slow hash is unnecessary.
pub fn hash_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(token.as_bytes());
    hex::encode(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_key_has_prefix() {
        let key = generate_api_key();
        assert!(key.starts_with("sk_"));
    }

    #[test]
    fn generate_key_has_sufficient_length() {
        let key = generate_api_key();
        // sk_ (3) + base64 of 32 bytes (43) = 46 chars
        assert!(key.len() >= 40);
    }

    #[test]
    fn generated_keys_are_unique() {
        let a = generate_api_key();
        let b = generate_api_key();
        assert_ne!(a, b);
    }

    #[test]
    fn hash_is_deterministic() {
        let key = "sk_test_token_123";
        assert_eq!(hash_token(key), hash_token(key));
    }

    #[test]
    fn hash_differs_for_different_inputs() {
        assert_ne!(hash_token("sk_aaa"), hash_token("sk_bbb"));
    }

    #[test]
    fn hash_is_hex_encoded_sha256() {
        let hash = hash_token("test");
        assert_eq!(hash.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn prefix_from_key() {
        let key = generate_api_key();
        let prefix = ApiKeyPrefix::from_key(&key);
        assert_eq!(prefix.as_ref().len(), DISPLAY_PREFIX_LEN);
        assert!(key.starts_with(prefix.as_ref()));
    }

    #[test]
    fn prefix_display() {
        let prefix = ApiKeyPrefix::from_key("sk_abcdefghij_rest");
        assert_eq!(format!("{}", prefix), "sk_abcdefg...");
    }

    #[test]
    fn prefix_from_short_key() {
        let prefix = ApiKeyPrefix::from_key("sk_ab");
        assert_eq!(prefix.as_ref(), "sk_ab");
    }
}
