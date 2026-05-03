use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum TunnelIdError {
    #[error("tunnel ID cannot be empty")]
    Empty,

    #[error("tunnel ID must be at least 3 characters, got {0}")]
    TooShort(usize),

    #[error("tunnel ID must be at most 63 characters, got {0}")]
    TooLong(usize),

    #[error("tunnel ID must be lowercase")]
    NotLowercase,

    #[error("tunnel ID must contain only lowercase letters, numbers, and hyphens, and cannot start or end with a hyphen")]
    InvalidFormat,
}

const DOMAIN_SAFE_ALPHABET: &[char] = &[
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h',
    'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
];

const DEFAULT_LENGTH: usize = 8;
const MIN_LENGTH: usize = 3;
const MAX_LENGTH: usize = 63;

/// a validated tunnel identifier, guaranteed to be a valid dns subdomain label.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct TunnelId(String);

impl TunnelId {
    pub fn new(raw: impl Into<String>) -> Result<Self, TunnelIdError> {
        let s = raw.into();
        Self::validate(&s)?;
        Ok(Self(s))
    }

    pub fn generate() -> Self {
        let id = nanoid::nanoid!(DEFAULT_LENGTH, DOMAIN_SAFE_ALPHABET);
        Self(id)
    }

    fn validate(s: &str) -> Result<(), TunnelIdError> {
        if s.is_empty() {
            return Err(TunnelIdError::Empty);
        }
        if s.len() < MIN_LENGTH {
            return Err(TunnelIdError::TooShort(s.len()));
        }
        if s.len() > MAX_LENGTH {
            return Err(TunnelIdError::TooLong(s.len()));
        }
        if s != s.to_lowercase() {
            return Err(TunnelIdError::NotLowercase);
        }

        let bytes = s.as_bytes();
        let is_alnum = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit();
        let is_label_char = |b: u8| is_alnum(b) || b == b'-';

        if !is_alnum(bytes[0])
            || !is_alnum(bytes[bytes.len() - 1])
            || !bytes[1..bytes.len() - 1].iter().all(|&b| is_label_char(b))
        {
            return Err(TunnelIdError::InvalidFormat);
        }

        Ok(())
    }
}

impl AsRef<str> for TunnelId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TunnelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<TunnelId> for String {
    fn from(id: TunnelId) -> Self {
        id.0
    }
}

impl TryFrom<String> for TunnelId {
    type Error = TunnelIdError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_ids() {
        assert!(TunnelId::new("abc").is_ok());
        assert!(TunnelId::new("my-tunnel").is_ok());
        assert!(TunnelId::new("test-123").is_ok());
        assert!(TunnelId::new("a1b").is_ok());
        assert!(TunnelId::new("a".repeat(MAX_LENGTH)).is_ok());
    }

    #[test]
    fn empty_id() {
        assert!(matches!(TunnelId::new(""), Err(TunnelIdError::Empty)));
    }

    #[test]
    fn too_short() {
        assert!(matches!(TunnelId::new("ab"), Err(TunnelIdError::TooShort(2))));
        assert!(matches!(TunnelId::new("a"), Err(TunnelIdError::TooShort(1))));
    }

    #[test]
    fn too_long() {
        let long = "a".repeat(MAX_LENGTH + 1);
        assert!(matches!(
            TunnelId::new(long),
            Err(TunnelIdError::TooLong(64))
        ));
    }

    #[test]
    fn uppercase_rejected() {
        assert!(matches!(
            TunnelId::new("MyTunnel"),
            Err(TunnelIdError::NotLowercase)
        ));
    }

    #[test]
    fn invalid_format() {
        assert!(matches!(TunnelId::new("-abc"), Err(TunnelIdError::InvalidFormat)));
        assert!(matches!(TunnelId::new("abc-"), Err(TunnelIdError::InvalidFormat)));
        assert!(matches!(TunnelId::new("ab_c"), Err(TunnelIdError::InvalidFormat)));
        assert!(matches!(TunnelId::new("ab c"), Err(TunnelIdError::InvalidFormat)));
    }

    #[test]
    fn generate_produces_valid_id() {
        for _ in 0..100 {
            let id = TunnelId::generate();
            assert_eq!(id.as_ref().len(), DEFAULT_LENGTH);
            assert!(TunnelId::new(id.as_ref()).is_ok());
        }
    }

    #[test]
    fn display_and_into_string() {
        let id = TunnelId::new("my-tunnel").unwrap();
        assert_eq!(id.to_string(), "my-tunnel");
        let s: String = id.into();
        assert_eq!(s, "my-tunnel");
    }

    #[test]
    fn serde_roundtrip() {
        let id = TunnelId::new("test-123").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"test-123\"");

        let parsed: TunnelId = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.as_ref(), "test-123");
    }

    #[test]
    fn serde_rejects_invalid() {
        let result: Result<TunnelId, _> = serde_json::from_str("\"AB\"");
        assert!(result.is_err());
    }
}
