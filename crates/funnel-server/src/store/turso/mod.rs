pub mod account_store;
pub mod api_key_store;
pub mod session_recorder;
pub mod team_store;
pub mod user_store;

use std::sync::Arc;

use chrono::{DateTime, Utc};
use turso::Database;
use uuid::Uuid;

use super::StoreError;

const SCHEMA: &str = include_str!("../../../../../migrations/001_turso.sql");

pub async fn open(path: &str) -> Result<Arc<Database>, StoreError> {
    let db = turso::Builder::new_local(path)
        .build()
        .await
        .map_err(|e| map_err(&e))?;
    let conn = db.connect().map_err(|e| map_err(&e))?;
    conn.execute_batch(SCHEMA).await.map_err(|e| map_err(&e))?;
    Ok(Arc::new(db))
}

pub fn map_err(e: &turso::Error) -> StoreError {
    let msg = e.to_string();
    if msg.contains("UNIQUE constraint failed") {
        StoreError::Conflict(msg)
    } else {
        StoreError::Other(msg)
    }
}

pub fn parse_uuid(s: &str) -> Result<Uuid, StoreError> {
    s.parse::<Uuid>()
        .map_err(|e| StoreError::Other(format!("invalid uuid: {e}")))
}

pub fn format_dt(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

pub fn parse_dt(s: &str) -> Result<DateTime<Utc>, StoreError> {
    s.parse::<DateTime<Utc>>()
        .map_err(|e| StoreError::Other(format!("invalid datetime: {e}")))
}

pub fn parse_optional_dt(s: Option<String>) -> Result<Option<DateTime<Utc>>, StoreError> {
    match s {
        Some(s) if !s.is_empty() => Ok(Some(parse_dt(&s)?)),
        _ => Ok(None),
    }
}
