use thiserror::Error;

#[derive(Debug, Error)]
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
