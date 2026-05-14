pub mod account_store;
pub mod api_key_store;
pub mod health;
pub mod pg;
pub mod session_recorder;
pub mod team_store;
pub mod tunnel_registry;
pub mod turso;
pub mod user_store;

use std::fmt;

#[derive(Debug)]
pub enum StoreError {
    Database(sqlx::Error),
    NotFound,
    Conflict(String),
    Other(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(e) => write!(f, "database error: {e}"),
            Self::NotFound => write!(f, "not found"),
            Self::Conflict(msg) => write!(f, "conflict: {msg}"),
            Self::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(e) => Some(e),
            _ => None,
        }
    }
}

impl From<sqlx::Error> for StoreError {
    fn from(e: sqlx::Error) -> Self {
        if let sqlx::Error::Database(ref db_err) = e
            && db_err.is_unique_violation()
        {
            return Self::Conflict(db_err.message().to_string());
        }
        Self::Database(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_not_found() {
        assert_eq!(StoreError::NotFound.to_string(), "not found");
    }

    #[test]
    fn display_conflict() {
        let err = StoreError::Conflict("duplicate key".into());
        assert_eq!(err.to_string(), "conflict: duplicate key");
    }

    #[test]
    fn display_other() {
        let err = StoreError::Other("something went wrong".into());
        assert_eq!(err.to_string(), "something went wrong");
    }

    #[test]
    fn display_database() {
        let sqlx_err = sqlx::Error::RowNotFound;
        let err = StoreError::Database(sqlx_err);
        let display = err.to_string();
        assert!(display.starts_with("database error:"), "got: {display}");
    }

    #[test]
    fn from_sqlx_row_not_found_becomes_database() {
        let err: StoreError = sqlx::Error::RowNotFound.into();
        assert!(matches!(err, StoreError::Database(_)));
    }

    #[test]
    fn from_sqlx_protocol_becomes_database() {
        let err: StoreError = sqlx::Error::Protocol("bad".into()).into();
        assert!(matches!(err, StoreError::Database(_)));
    }

    #[test]
    fn source_returns_inner_for_database() {
        let sqlx_err = sqlx::Error::RowNotFound;
        let err = StoreError::Database(sqlx_err);
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn source_returns_none_for_non_database() {
        assert!(std::error::Error::source(&StoreError::NotFound).is_none());
        assert!(std::error::Error::source(&StoreError::Conflict("x".into())).is_none());
        assert!(std::error::Error::source(&StoreError::Other("x".into())).is_none());
    }
}
