use std::sync::Arc;

use chrono::{DateTime, Utc};
use turso::Database;
use uuid::Uuid;

use super::{format_dt, map_err, parse_dt, parse_optional_dt, parse_uuid};
use crate::db::api_keys::{ApiKey, ApiKeyView};
use crate::store::api_key_store::ApiKeyStore;
use crate::store::StoreError;
use funnel_core::api::ApiScope;
use funnel_core::auth::token as auth;

pub struct TursoApiKeyStore {
    db: Arc<Database>,
}

impl TursoApiKeyStore {
    pub const fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

fn row_to_api_key(row: &turso::Row) -> Result<ApiKey, StoreError> {
    Ok(ApiKey {
        id: parse_uuid(&row.get::<String>(0).map_err(|e| map_err(&e))?)?,
        user_id: parse_uuid(&row.get::<String>(1).map_err(|e| map_err(&e))?)?,
        name: row.get::<String>(2).map_err(|e| map_err(&e))?,
        key_hash: row.get::<String>(3).map_err(|e| map_err(&e))?,
        key_prefix: row.get::<String>(4).map_err(|e| map_err(&e))?,
        scopes: serde_json::from_str(&row.get::<String>(5).map_err(|e| map_err(&e))?)
            .map_err(|e| StoreError::Other(format!("invalid json: {e}")))?,
        created_at: parse_dt(&row.get::<String>(6).map_err(|e| map_err(&e))?)?,
        revoked_at: parse_optional_dt(row.get::<Option<String>>(7).map_err(|e| map_err(&e))?)?,
        expires_at: parse_optional_dt(row.get::<Option<String>>(8).map_err(|e| map_err(&e))?)?,
    })
}

#[async_trait::async_trait]
impl ApiKeyStore for TursoApiKeyStore {
    async fn create(
        &self,
        user_id: Uuid,
        name: &str,
        scopes: &[ApiScope],
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<(String, ApiKeyView), StoreError> {
        let plaintext = auth::generate_api_key()
            .map_err(|e| StoreError::Other(format!("failed to generate api key: {e}")))?;
        let hash = auth::hash_token(&plaintext);
        let prefix = auth::ApiKeyPrefix::from_key(&plaintext);

        let conn = self.db.connect().map_err(|e| map_err(&e))?;
        let id = Uuid::now_v7();
        let now = Utc::now();
        let scopes_str = serde_json::to_string(scopes)
            .map_err(|e| StoreError::Other(format!("invalid json: {e}")))?;
        let expires_str = expires_at.map(format_dt);

        conn.execute(
            "INSERT INTO api_keys (id, user_id, name, key_hash, key_prefix, scopes, created_at, expires_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            turso::params![
                id.to_string(),
                user_id.to_string(),
                name.to_string(),
                hash,
                prefix.as_ref().to_string(),
                scopes_str,
                format_dt(now),
                expires_str.unwrap_or_default()
            ],
        )
        .await
        .map_err(|e| map_err(&e))?;

        let view = ApiKeyView {
            id,
            name: name.to_string(),
            key_prefix: prefix.as_ref().to_string(),
            scopes: scopes.to_vec(),
            created_at: now,
            revoked_at: None,
            expires_at,
        };

        Ok((plaintext, view))
    }

    async fn validate(&self, plaintext: &str) -> Result<Option<ApiKey>, StoreError> {
        let hash = auth::hash_token(plaintext);
        let conn = self.db.connect().map_err(|e| map_err(&e))?;
        let now = format_dt(Utc::now());
        let mut rows = conn
            .query(
                "SELECT id, user_id, name, key_hash, key_prefix, scopes, created_at, revoked_at, expires_at FROM api_keys WHERE key_hash = ? AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at = '' OR expires_at > ?)",
                turso::params![hash, now],
            )
            .await
            .map_err(|e| map_err(&e))?;
        match rows.next().await.map_err(|e| map_err(&e))? {
            Some(row) => Ok(Some(row_to_api_key(&row)?)),
            None => Ok(None),
        }
    }

    async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<ApiKeyView>, StoreError> {
        let conn = self.db.connect().map_err(|e| map_err(&e))?;
        let mut rows = conn
            .query(
                "SELECT id, user_id, name, key_hash, key_prefix, scopes, created_at, revoked_at, expires_at FROM api_keys WHERE user_id = ? AND revoked_at IS NULL ORDER BY created_at DESC",
                turso::params![user_id.to_string()],
            )
            .await
            .map_err(|e| map_err(&e))?;
        let mut keys = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| map_err(&e))? {
            let key = row_to_api_key(&row)?;
            keys.push(ApiKeyView::from(key));
        }
        Ok(keys)
    }

    async fn revoke(&self, key_id: Uuid, user_id: Uuid) -> Result<bool, StoreError> {
        let conn = self.db.connect().map_err(|e| map_err(&e))?;
        let now = format_dt(Utc::now());
        let rows_affected = conn
            .execute(
                "UPDATE api_keys SET revoked_at = ? WHERE id = ? AND user_id = ? AND revoked_at IS NULL",
                turso::params![now, key_id.to_string(), user_id.to_string()],
            )
            .await
            .map_err(|e| map_err(&e))?;
        Ok(rows_affected > 0)
    }

    async fn revoke_by_name(&self, user_id: Uuid, name: &str) -> Result<bool, StoreError> {
        let conn = self.db.connect().map_err(|e| map_err(&e))?;
        let now = format_dt(Utc::now());
        let rows_affected = conn
            .execute(
                "UPDATE api_keys SET revoked_at = ? WHERE user_id = ? AND name = ? AND revoked_at IS NULL",
                turso::params![now, user_id.to_string(), name.to_string()],
            )
            .await
            .map_err(|e| map_err(&e))?;
        Ok(rows_affected > 0)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::db::api_keys::default_scopes;
    use crate::db::users::NewUser;
    use crate::store::turso::open;
    use crate::store::turso::user_store::TursoUserStore;
    use crate::store::user_store::UserStore;

    async fn setup() -> (TursoApiKeyStore, Uuid) {
        let db = open(":memory:")
            .await
            .unwrap_or_else(|e| panic!("open: {e}"));
        let user_store = TursoUserStore::new(Arc::clone(&db));
        let user = user_store
            .create(NewUser {
                email: format!("key-test-{}@test.com", Uuid::now_v7()),
                name: None,
                avatar_url: None,
            })
            .await
            .unwrap_or_else(|e| panic!("create user: {e}"));
        (TursoApiKeyStore::new(db), user.id)
    }

    #[tokio::test]
    async fn create_and_validate() {
        let (store, uid) = setup().await;

        let (plaintext, view) = store
            .create(uid, "test-key", &default_scopes(), None)
            .await
            .unwrap();
        assert_eq!(view.name, "test-key");

        let found = store.validate(&plaintext).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, view.id);
    }

    #[tokio::test]
    async fn validate_wrong_key_returns_none() {
        let (store, _uid) = setup().await;
        let found = store.validate("fnl_boguskey1234567890").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn duplicate_name_returns_conflict() {
        let (store, uid) = setup().await;

        store
            .create(uid, "dup", &default_scopes(), None)
            .await
            .unwrap();
        let result = store.create(uid, "dup", &default_scopes(), None).await;
        assert!(matches!(result, Err(StoreError::Conflict(_))));
    }

    #[tokio::test]
    async fn list_for_user_only_returns_matching() {
        let db = open(":memory:")
            .await
            .unwrap_or_else(|e| panic!("open: {e}"));
        let user_store = TursoUserStore::new(Arc::clone(&db));
        let u1 = user_store
            .create(NewUser {
                email: format!("u1-{}@t.com", Uuid::now_v7()),
                name: None,
                avatar_url: None,
            })
            .await
            .unwrap();
        let u2 = user_store
            .create(NewUser {
                email: format!("u2-{}@t.com", Uuid::now_v7()),
                name: None,
                avatar_url: None,
            })
            .await
            .unwrap();
        let store = TursoApiKeyStore::new(db);

        store
            .create(u1.id, "a", &default_scopes(), None)
            .await
            .unwrap();
        store
            .create(u1.id, "b", &default_scopes(), None)
            .await
            .unwrap();
        store
            .create(u2.id, "c", &default_scopes(), None)
            .await
            .unwrap();

        let list = store.list_for_user(u1.id).await.unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().all(|k| k.name == "a" || k.name == "b"));
    }

    #[tokio::test]
    async fn revoke_makes_key_invalid() {
        let (store, uid) = setup().await;

        let (plaintext, view) = store
            .create(uid, "to-revoke", &default_scopes(), None)
            .await
            .unwrap();
        let revoked = store.revoke(view.id, uid).await.unwrap();
        assert!(revoked);

        let found = store.validate(&plaintext).await.unwrap();
        assert!(found.is_none());

        let list = store.list_for_user(uid).await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn revoke_wrong_user_returns_false() {
        let (store, uid) = setup().await;
        let other = Uuid::now_v7();

        let (_plaintext, view) = store
            .create(uid, "key", &default_scopes(), None)
            .await
            .unwrap();
        let revoked = store.revoke(view.id, other).await.unwrap();
        assert!(!revoked);
    }

    #[tokio::test]
    async fn can_reuse_name_after_revoke() {
        let (store, uid) = setup().await;

        let (_plaintext, view) = store
            .create(uid, "reuse", &default_scopes(), None)
            .await
            .unwrap();
        store.revoke(view.id, uid).await.unwrap();

        let result = store.create(uid, "reuse", &default_scopes(), None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn scopes_are_preserved() {
        let (store, uid) = setup().await;
        let scopes = &[ApiScope::Tunnels];

        let (plaintext, view) = store
            .create(uid, "tunnel-only", scopes, None)
            .await
            .unwrap();
        assert_eq!(view.scopes, vec![ApiScope::Tunnels]);

        let key = store.validate(&plaintext).await.unwrap().unwrap();
        assert!(key.has_scope(ApiScope::Tunnels));
        assert!(!key.has_scope(ApiScope::Management));
    }

    #[tokio::test]
    async fn expired_key_is_rejected() {
        let (store, uid) = setup().await;
        let past = Utc::now() - chrono::Duration::hours(1);

        let (plaintext, _) = store
            .create(uid, "expired", &default_scopes(), Some(past))
            .await
            .unwrap();

        let found = store.validate(&plaintext).await.unwrap();
        assert!(found.is_none());
    }
}
