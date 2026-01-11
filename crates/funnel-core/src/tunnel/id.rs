use std::fmt;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::ValidationError;

const DOMAIN_SAFE_ALPHABET: &[char] = &[
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h',
    'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
];

const DEFAULT_ID_LENGTH: usize = 8;

/// A validated tunnel identifier.
///
/// Guaranteed to be 3-63 characters, lowercase alphanumeric with hyphens,
/// and not starting or ending with a hyphen. Safe for use as a DNS subdomain label.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct TunnelId(String);

impl TunnelId {
    /// Parse and validate a raw string as a tunnel ID.
    pub fn new(raw: impl Into<String>) -> Result<Self, ValidationError> {
        let s = raw.into();
        Self::validate(&s)?;
        Ok(Self(s))
    }

    /// Generate a random domain-safe tunnel ID.
    pub fn generate() -> Self {
        let id = nanoid::nanoid!(DEFAULT_ID_LENGTH, DOMAIN_SAFE_ALPHABET);
        Self(id)
    }

    fn validate(s: &str) -> Result<(), ValidationError> {
        if s.is_empty() {
            return Err(ValidationError::Empty);
        }
        if s.len() < 3 {
            return Err(ValidationError::TooShort(s.len()));
        }
        if s.len() > 63 {
            return Err(ValidationError::TooLong(s.len()));
        }

        let lower = s.to_lowercase();
        if s != lower {
            return Err(ValidationError::NotLowercase);
        }

        // Must match: starts and ends with alnum, middle can have hyphens
        let re = Regex::new(r"^[a-z0-9][a-z0-9-]*[a-z0-9]$").expect("valid regex");
        if !re.is_match(s) {
            return Err(ValidationError::InvalidFormat);
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
    type Error = ValidationError;

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
        assert!(TunnelId::new("a".repeat(63)).is_ok());
    }

    #[test]
    fn empty_id() {
        assert!(matches!(TunnelId::new(""), Err(ValidationError::Empty)));
    }

    #[test]
    fn too_short() {
        assert!(matches!(TunnelId::new("ab"), Err(ValidationError::TooShort(2))));
        assert!(matches!(TunnelId::new("a"), Err(ValidationError::TooShort(1))));
    }

    #[test]
    fn too_long() {
        let long = "a".repeat(64);
        assert!(matches!(TunnelId::new(long), Err(ValidationError::TooLong(64))));
    }

    #[test]
    fn uppercase_rejected() {
        assert!(matches!(
            TunnelId::new("MyTunnel"),
            Err(ValidationError::NotLowercase)
        ));
    }

    #[test]
    fn invalid_format() {
        assert!(matches!(
            TunnelId::new("-abc"),
            Err(ValidationError::InvalidFormat)
        ));
        assert!(matches!(
            TunnelId::new("abc-"),
            Err(ValidationError::InvalidFormat)
        ));
        assert!(matches!(
            TunnelId::new("ab_c"),
            Err(ValidationError::InvalidFormat)
        ));
        assert!(matches!(
            TunnelId::new("ab c"),
            Err(ValidationError::InvalidFormat)
        ));
    }

    #[test]
    fn generate_produces_valid_id() {
        for _ in 0..100 {
            let id = TunnelId::generate();
            assert_eq!(id.as_ref().len(), DEFAULT_ID_LENGTH);
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
