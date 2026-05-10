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

    fn create(&self, new_user: NewUser) -> BoxFuture<'_, Result<User, StoreError>> {
        Box::pin(async move {
            let now = Utc::now();
            let user = User {
                id: Uuid::now_v7(),
                email: new_user.email,
                name: new_user.name,
                avatar_url: new_user.avatar_url,
                role: "user".into(),
                metadata: serde_json::json!({}),
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

    fn update_profile(
        &self,
        id: Uuid,
        name: Option<&str>,
        avatar_url: Option<&str>,
    ) -> BoxFuture<'_, Result<User, StoreError>> {
        let name = name.map(ToString::to_string);
        let avatar_url = avatar_url.map(ToString::to_string);
        Box::pin(async move {
            let mut users = self.users.write().unwrap_or_else(PoisonError::into_inner);
            let user = users
                .iter_mut()
                .find(|u| u.id == id)
                .ok_or(StoreError::NotFound)?;
            user.name = name;
            user.avatar_url = avatar_url;
            user.updated_at = Utc::now();
            let snapshot = user.clone();
            drop(users);
            Ok(snapshot)
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
    async fn update_profile_changes_fields() {
        let store = InMemoryUserStore::new();
        let user = store.create(make_new_user("c@test.com")).await.unwrap();

        let updated = store
            .update_profile(user.id, Some("New Name"), Some("https://img.test/1.png"))
            .await
            .unwrap();
        assert_eq!(updated.name, Some("New Name".into()));
        assert_eq!(updated.avatar_url, Some("https://img.test/1.png".into()));
        assert!(updated.updated_at > user.updated_at);
    }

    #[tokio::test]
    async fn new_user_gets_default_role() {
        let store = InMemoryUserStore::new();
        let user = store.create(make_new_user("d@test.com")).await.unwrap();
        assert_eq!(user.role, "user");
    }
}
