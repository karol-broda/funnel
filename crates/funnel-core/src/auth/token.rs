use std::fmt;

use base64::prelude::*;
use serde::{Deserialize, Serialize};

const PREFIX: &str = "sk_";
const RANDOM_BYTES: usize = 32;
const VISIBLE_PREFIX_LEN: usize = 10;

/// the visible prefix of an api key, safe for display and logging.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiKeyPrefix(String);

impl ApiKeyPrefix {
    pub fn from_key(full_key: &str) -> Self {
        let end = full_key.len().min(VISIBLE_PREFIX_LEN);
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

pub fn generate_api_key() -> Result<String, getrandom::Error> {
    let mut bytes = [0u8; RANDOM_BYTES];
    getrandom::fill(&mut bytes)?;
    Ok(format!(
        "{}{}",
        PREFIX,
        BASE64_URL_SAFE_NO_PAD.encode(bytes)
    ))
}

/// sha256 is fine here since api keys are high entropy random tokens,
/// not user chosen passwords, so slow hashing is unnecessary.
pub fn hash_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(token.as_bytes());
    hex::encode(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_key_has_prefix() -> Result<(), getrandom::Error> {
        let key = generate_api_key()?;
        assert!(key.starts_with(PREFIX));
        Ok(())
    }

    #[test]
    fn generate_key_has_sufficient_length() -> Result<(), getrandom::Error> {
        let key = generate_api_key()?;
        // prefix (3) + base64 of 32 bytes (43) = 46 chars
        assert!(key.len() >= 40);
        Ok(())
    }

    #[test]
    fn generated_keys_are_unique() -> Result<(), getrandom::Error> {
        let a = generate_api_key()?;
        let b = generate_api_key()?;
        assert_ne!(a, b);
        Ok(())
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
        // 32 bytes = 64 hex chars
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn prefix_from_key() -> Result<(), getrandom::Error> {
        let key = generate_api_key()?;
        let prefix = ApiKeyPrefix::from_key(&key);
        assert_eq!(prefix.as_ref().len(), VISIBLE_PREFIX_LEN);
        assert!(key.starts_with(prefix.as_ref()));
        Ok(())
    }

    #[test]
    fn prefix_display() {
        let prefix = ApiKeyPrefix::from_key("sk_abcdefghij_rest");
        assert_eq!(format!("{prefix}"), "sk_abcdefg...");
    }

    #[test]
    fn prefix_from_short_key() {
        let prefix = ApiKeyPrefix::from_key("sk_ab");
        assert_eq!(prefix.as_ref(), "sk_ab");
    }
}
