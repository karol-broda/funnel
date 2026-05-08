use std::sync::{PoisonError, RwLock};

use chrono::Utc;
use uuid::Uuid;

use crate::db::users::{NewUser, User};
use crate::store::user_store::UserStore;
use crate::store::{BoxFuture, StoreError};

pub struct InMemoryUserStore {
    users: RwLock<Vec<User>>,
}

impl InMemoryUserStore {
    pub const fn new() -> Self {
        Self {
            users: RwLock::new(Vec::new()),
        }
    }
}

impl UserStore for InMemoryUserStore {
    fn find_by_id(&self, id: Uuid) -> BoxFuture<'_, Result<Option<User>, StoreError>> {
        Box::pin(async move {
            let users = self.users.read().unwrap_or_else(PoisonError::into_inner);
            Ok(users.iter().find(|u| u.id == id).cloned())
        })
    }

    fn find_by_email(&self, email: &str) -> BoxFuture<'_, Result<Option<User>, StoreError>> {
        let email = email.to_string();
        Box::pin(async move {
            let users = self.users.read().unwrap_or_else(PoisonError::into_inner);
            Ok(users.iter().find(|u| u.email == email).cloned())
        })
    }

    fn find_by_provider(&self, provider: &str, provider_id: &str) -> BoxFuture<'_, Result<Option<User>, StoreError>> {
        let provider = provider.to_string();
        let provider_id = provider_id.to_string();
        Box::pin(async move {
            let users = self.users.read().unwrap_or_else(PoisonError::into_inner);
            Ok(users.iter().find(|u| u.provider == provider && u.provider_id == provider_id).cloned())
        })
    }

    fn create(&self, new_user: NewUser) -> BoxFuture<'_, Result<User, StoreError>> {
        Box::pin(async move {
            let now = Utc::now();
            let user = User {
                id: Uuid::now_v7(),
                email: new_user.email,
                name: new_user.name,
                avatar_url: new_user.avatar_url,
                provider: new_user.provider,
                provider_id: new_user.provider_id,
                created_at: now,
                updated_at: now,
            };
            {
                let mut users = self.users.write().unwrap_or_else(PoisonError::into_inner);
                users.push(user.clone());
            }
            Ok(user)
        })
    }

    fn upsert_from_oauth(&self, new_user: NewUser) -> BoxFuture<'_, Result<User, StoreError>> {
        Box::pin(async move {
            let mut users = self.users.write().unwrap_or_else(PoisonError::into_inner);
            let result = if let Some(existing) = users.iter_mut().find(|u| u.provider == new_user.provider && u.provider_id == new_user.provider_id) {
                existing.email = new_user.email;
                existing.name = new_user.name;
                existing.avatar_url = new_user.avatar_url;
                existing.updated_at = Utc::now();
                existing.clone()
            } else {
                let now = Utc::now();
                let user = User {
                    id: Uuid::now_v7(),
                    email: new_user.email,
                    name: new_user.name,
                    avatar_url: new_user.avatar_url,
                    provider: new_user.provider,
                    provider_id: new_user.provider_id,
                    created_at: now,
                    updated_at: now,
                };
                users.push(user.clone());
                user
            };
            drop(users);
            Ok(result)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_new_user(email: &str) -> NewUser {
        NewUser {
            email: email.to_string(),
            name: Some("Test User".to_string()),
            avatar_url: None,
            provider: "github".to_string(),
            provider_id: format!("gh_{email}"),
        }
    }

    #[tokio::test]
    async fn create_and_find_by_id() {
        let store = InMemoryUserStore::new();
        let user = store.create(make_new_user("a@test.com")).await.unwrap();

        let found = store.find_by_id(user.id).await.unwrap();
        assert_eq!(found.unwrap().email, "a@test.com");
    }

    #[tokio::test]
    async fn find_by_id_missing_returns_none() {
        let store = InMemoryUserStore::new();
        let found = store.find_by_id(Uuid::now_v7()).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn find_by_email() {
        let store = InMemoryUserStore::new();
        store.create(make_new_user("b@test.com")).await.unwrap();

        let found = store.find_by_email("b@test.com").await.unwrap();
        assert!(found.is_some());

        let missing = store.find_by_email("nope@test.com").await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn find_by_provider() {
        let store = InMemoryUserStore::new();
        store.create(make_new_user("c@test.com")).await.unwrap();

        let found = store.find_by_provider("github", "gh_c@test.com").await.unwrap();
        assert!(found.is_some());

        let missing = store.find_by_provider("github", "gh_nope").await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn upsert_creates_new_user() {
        let store = InMemoryUserStore::new();
        let user = store.upsert_from_oauth(make_new_user("d@test.com")).await.unwrap();
        assert_eq!(user.email, "d@test.com");

        let found = store.find_by_id(user.id).await.unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn upsert_updates_existing_user() {
        let store = InMemoryUserStore::new();
        let original = store.upsert_from_oauth(make_new_user("e@test.com")).await.unwrap();

        let mut updated_input = make_new_user("new_e@test.com");
        updated_input.provider = "github".to_string();
        updated_input.provider_id = "gh_e@test.com".to_string();
        updated_input.name = Some("Updated Name".to_string());

        let updated = store.upsert_from_oauth(updated_input).await.unwrap();
        assert_eq!(updated.id, original.id);
        assert_eq!(updated.email, "new_e@test.com");
        assert_eq!(updated.name, Some("Updated Name".to_string()));
        assert!(updated.updated_at > original.updated_at);
    }
}
