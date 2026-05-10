use std::sync::{PoisonError, RwLock};

use chrono::Utc;
use uuid::Uuid;

use crate::db::api_keys::{ApiKey, ApiKeyView};
use crate::store::api_key_store::ApiKeyStore;
use crate::store::{BoxFuture, StoreError};
use funnel_core::auth::token as auth;

pub struct InMemoryApiKeyStore {
    keys: RwLock<Vec<ApiKey>>,
}

impl InMemoryApiKeyStore {
    pub const fn new() -> Self {
        Self {
            keys: RwLock::new(Vec::new()),
        }
    }
}

impl ApiKeyStore for InMemoryApiKeyStore {
    fn create(
        &self,
        user_id: Uuid,
        name: &str,
        scopes: &serde_json::Value,
    ) -> BoxFuture<'_, Result<(String, ApiKeyView), StoreError>> {
        let name = name.to_string();
        let scopes = scopes.clone();
        Box::pin(async move {
            let plaintext = auth::generate_api_key()
                .map_err(|e| StoreError::Other(format!("failed to generate api key: {e}")))?;
            let hash = auth::hash_token(&plaintext);
            let prefix = auth::ApiKeyPrefix::from_key(&plaintext);

            let key = ApiKey {
                id: Uuid::now_v7(),
                user_id,
                name: name.clone(),
                key_hash: hash,
                key_prefix: prefix.as_ref().to_string(),
                scopes,
                created_at: Utc::now(),
                revoked_at: None,
            };

            let view = {
                let mut keys = self.keys.write().unwrap_or_else(PoisonError::into_inner);

                if keys
                    .iter()
                    .any(|k| k.user_id == user_id && k.name == name && k.revoked_at.is_none())
                {
                    return Err(StoreError::Conflict(format!(
                        "api key name already exists: {name}"
                    )));
                }

                let view = ApiKeyView::from(key.clone());
                keys.push(key);
                view
            };

            Ok((plaintext, view))
        })
    }

    fn validate(&self, plaintext: &str) -> BoxFuture<'_, Result<Option<ApiKey>, StoreError>> {
        let hash = auth::hash_token(plaintext);
        Box::pin(async move {
            let keys = self.keys.read().unwrap_or_else(PoisonError::into_inner);
            Ok(keys
                .iter()
                .find(|k| k.key_hash == hash && k.revoked_at.is_none())
                .cloned())
        })
    }

    fn list_for_user(&self, user_id: Uuid) -> BoxFuture<'_, Result<Vec<ApiKeyView>, StoreError>> {
        Box::pin(async move {
            let keys = self.keys.read().unwrap_or_else(PoisonError::into_inner);
            Ok(keys
                .iter()
                .filter(|k| k.user_id == user_id && k.revoked_at.is_none())
                .cloned()
                .map(Into::into)
                .collect())
        })
    }

    fn revoke(&self, key_id: Uuid, user_id: Uuid) -> BoxFuture<'_, Result<bool, StoreError>> {
        Box::pin(async move {
            let mut keys = self.keys.write().unwrap_or_else(PoisonError::into_inner);
            if let Some(key) = keys
                .iter_mut()
                .find(|k| k.id == key_id && k.user_id == user_id && k.revoked_at.is_none())
            {
                key.revoked_at = Some(Utc::now());
                Ok(true)
            } else {
                Ok(false)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::api_keys::default_scopes;

    fn user_id() -> Uuid {
        Uuid::now_v7()
    }

    #[tokio::test]
    async fn create_and_validate() {
        let store = InMemoryApiKeyStore::new();
        let uid = user_id();

        let (plaintext, view) = store
            .create(uid, "test-key", &default_scopes())
            .await
            .unwrap();
        assert_eq!(view.name, "test-key");

        let found = store.validate(&plaintext).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, view.id);
    }

    #[tokio::test]
    async fn validate_wrong_key_returns_none() {
        let store = InMemoryApiKeyStore::new();
        let found = store.validate("fnl_boguskey1234567890").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn duplicate_name_returns_conflict() {
        let store = InMemoryApiKeyStore::new();
        let uid = user_id();

        store.create(uid, "dup", &default_scopes()).await.unwrap();
        let result = store.create(uid, "dup", &default_scopes()).await;
        assert!(matches!(result, Err(StoreError::Conflict(_))));
    }

    #[tokio::test]
    async fn list_for_user_only_returns_matching() {
        let store = InMemoryApiKeyStore::new();
        let uid1 = user_id();
        let uid2 = user_id();

        store.create(uid1, "a", &default_scopes()).await.unwrap();
        store.create(uid1, "b", &default_scopes()).await.unwrap();
        store.create(uid2, "c", &default_scopes()).await.unwrap();

        let list = store.list_for_user(uid1).await.unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().all(|k| k.name == "a" || k.name == "b"));
    }

    #[tokio::test]
    async fn revoke_makes_key_invalid() {
        let store = InMemoryApiKeyStore::new();
        let uid = user_id();

        let (plaintext, view) = store
            .create(uid, "to-revoke", &default_scopes())
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
        let store = InMemoryApiKeyStore::new();
        let uid = user_id();
        let other = user_id();

        let (_plaintext, view) = store.create(uid, "key", &default_scopes()).await.unwrap();
        let revoked = store.revoke(view.id, other).await.unwrap();
        assert!(!revoked);
    }

    #[tokio::test]
    async fn can_reuse_name_after_revoke() {
        let store = InMemoryApiKeyStore::new();
        let uid = user_id();

        let (_plaintext, view) = store.create(uid, "reuse", &default_scopes()).await.unwrap();
        store.revoke(view.id, uid).await.unwrap();

        let result = store.create(uid, "reuse", &default_scopes()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn scopes_are_preserved() {
        let store = InMemoryApiKeyStore::new();
        let uid = user_id();
        let scopes = serde_json::json!(["tunnels"]);

        let (plaintext, view) = store.create(uid, "tunnel-only", &scopes).await.unwrap();
        assert_eq!(view.scopes, scopes);

        let key = store.validate(&plaintext).await.unwrap().unwrap();
        assert!(key.has_scope("tunnels"));
        assert!(!key.has_scope("management"));
    }
}
