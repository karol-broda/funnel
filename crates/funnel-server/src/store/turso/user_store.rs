use std::sync::Arc;

use chrono::Utc;
use turso::Database;
use uuid::Uuid;

use super::{format_dt, map_err, parse_dt, parse_optional_dt, parse_uuid};
use crate::db::users::{NewUser, User, ROLE_MEMBER};
use crate::store::user_store::UserStore;
use crate::store::{BoxFuture, StoreError};

pub struct TursoUserStore {
    db: Arc<Database>,
}

impl TursoUserStore {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

fn row_to_user(row: &turso::Row) -> Result<User, StoreError> {
    Ok(User {
        id: parse_uuid(&row.get::<String>(0).map_err(map_err)?)?,
        email: row.get::<String>(1).map_err(map_err)?,
        name: row.get::<Option<String>>(2).map_err(map_err)?,
        avatar_url: row.get::<Option<String>>(3).map_err(map_err)?,
        role: row.get::<String>(4).map_err(map_err)?,
        metadata: serde_json::from_str(&row.get::<String>(5).map_err(map_err)?)
            .map_err(|e| StoreError::Other(format!("invalid json: {e}")))?,
        created_at: parse_dt(&row.get::<String>(6).map_err(map_err)?)?,
        updated_at: parse_dt(&row.get::<String>(7).map_err(map_err)?)?,
        deactivated_at: parse_optional_dt(row.get::<Option<String>>(8).map_err(map_err)?)?,
    })
}

impl UserStore for TursoUserStore {
    fn find_by_id(&self, id: Uuid) -> BoxFuture<'_, Result<Option<User>, StoreError>> {
        Box::pin(async move {
            let conn = self.db.connect().map_err(map_err)?;
            let mut rows = conn
                .query(
                    "SELECT id, email, name, avatar_url, role, metadata, created_at, updated_at, deactivated_at FROM users WHERE id = ?",
                    turso::params![id.to_string()],
                )
                .await
                .map_err(map_err)?;
            match rows.next().await.map_err(map_err)? {
                Some(row) => Ok(Some(row_to_user(&row)?)),
                None => Ok(None),
            }
        })
    }

    fn find_by_email(&self, email: &str) -> BoxFuture<'_, Result<Option<User>, StoreError>> {
        let email = email.to_string();
        Box::pin(async move {
            let conn = self.db.connect().map_err(map_err)?;
            let mut rows = conn
                .query(
                    "SELECT id, email, name, avatar_url, role, metadata, created_at, updated_at, deactivated_at FROM users WHERE email = ?",
                    turso::params![email],
                )
                .await
                .map_err(map_err)?;
            match rows.next().await.map_err(map_err)? {
                Some(row) => Ok(Some(row_to_user(&row)?)),
                None => Ok(None),
            }
        })
    }

    fn create(&self, new_user: NewUser) -> BoxFuture<'_, Result<User, StoreError>> {
        Box::pin(async move {
            let conn = self.db.connect().map_err(map_err)?;
            let id = Uuid::now_v7();
            let now = Utc::now();
            let now_str = format_dt(now);
            let metadata = serde_json::json!({}).to_string();
            conn.execute(
                "INSERT INTO users (id, email, name, avatar_url, role, metadata, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                turso::params![
                    id.to_string(),
                    new_user.email.clone(),
                    new_user.name.clone().unwrap_or_default(),
                    new_user.avatar_url.clone().unwrap_or_default(),
                    ROLE_MEMBER,
                    metadata,
                    now_str.clone(),
                    now_str
                ],
            )
            .await
            .map_err(map_err)?;
            Ok(User {
                id,
                email: new_user.email,
                name: new_user.name,
                avatar_url: new_user.avatar_url,
                role: ROLE_MEMBER.into(),
                metadata: serde_json::json!({}),
                created_at: now,
                updated_at: now,
                deactivated_at: None,
            })
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
            let conn = self.db.connect().map_err(map_err)?;
            let now = format_dt(Utc::now());
            conn.execute(
                "UPDATE users SET name = ?, avatar_url = ?, updated_at = ? WHERE id = ?",
                turso::params![
                    name.clone().unwrap_or_default(),
                    avatar_url.clone().unwrap_or_default(),
                    now,
                    id.to_string()
                ],
            )
            .await
            .map_err(map_err)?;
            let user = self.find_by_id(id).await?.ok_or(StoreError::NotFound)?;
            Ok(user)
        })
    }

    fn update_role(&self, id: Uuid, role: &str) -> BoxFuture<'_, Result<User, StoreError>> {
        let role = role.to_string();
        Box::pin(async move {
            let conn = self.db.connect().map_err(map_err)?;
            let now = format_dt(Utc::now());
            conn.execute(
                "UPDATE users SET role = ?, updated_at = ? WHERE id = ?",
                turso::params![role, now, id.to_string()],
            )
            .await
            .map_err(map_err)?;
            let user = self.find_by_id(id).await?.ok_or(StoreError::NotFound)?;
            Ok(user)
        })
    }

    fn deactivate(&self, id: Uuid) -> BoxFuture<'_, Result<User, StoreError>> {
        Box::pin(async move {
            let conn = self.db.connect().map_err(map_err)?;
            let now = format_dt(Utc::now());
            conn.execute(
                "UPDATE users SET deactivated_at = ?, updated_at = ? WHERE id = ?",
                turso::params![now.clone(), now, id.to_string()],
            )
            .await
            .map_err(map_err)?;
            let user = self.find_by_id(id).await?.ok_or(StoreError::NotFound)?;
            Ok(user)
        })
    }

    fn reactivate(&self, id: Uuid) -> BoxFuture<'_, Result<User, StoreError>> {
        Box::pin(async move {
            let conn = self.db.connect().map_err(map_err)?;
            let now = format_dt(Utc::now());
            conn.execute(
                "UPDATE users SET deactivated_at = NULL, updated_at = ? WHERE id = ?",
                turso::params![now, id.to_string()],
            )
            .await
            .map_err(map_err)?;
            let user = self.find_by_id(id).await?.ok_or(StoreError::NotFound)?;
            Ok(user)
        })
    }

    fn list_all(&self, limit: i64) -> BoxFuture<'_, Result<Vec<User>, StoreError>> {
        Box::pin(async move {
            let conn = self.db.connect().map_err(map_err)?;
            let mut rows = conn
                .query(
                    "SELECT id, email, name, avatar_url, role, metadata, created_at, updated_at, deactivated_at FROM users ORDER BY created_at DESC LIMIT ?",
                    turso::params![limit],
                )
                .await
                .map_err(map_err)?;
            let mut users = Vec::new();
            while let Some(row) = rows.next().await.map_err(map_err)? {
                users.push(row_to_user(&row)?);
            }
            Ok(users)
        })
    }

    fn count(&self) -> BoxFuture<'_, Result<i64, StoreError>> {
        Box::pin(async move {
            let conn = self.db.connect().map_err(map_err)?;
            let mut rows = conn
                .query("SELECT COUNT(*) FROM users", ())
                .await
                .map_err(map_err)?;
            let row = rows
                .next()
                .await
                .map_err(map_err)?
                .ok_or(StoreError::Other("no count row".into()))?;
            Ok(row.get::<i64>(0).map_err(map_err)?)
        })
    }

    fn count_admins(&self) -> BoxFuture<'_, Result<i64, StoreError>> {
        Box::pin(async move {
            let conn = self.db.connect().map_err(map_err)?;
            let mut rows = conn
                .query(
                    "SELECT COUNT(*) FROM users WHERE role = 'admin' AND deactivated_at IS NULL",
                    (),
                )
                .await
                .map_err(map_err)?;
            let row = rows
                .next()
                .await
                .map_err(map_err)?
                .ok_or(StoreError::Other("no count row".into()))?;
            Ok(row.get::<i64>(0).map_err(map_err)?)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::turso::open;

    async fn store() -> TursoUserStore {
        let db = open(":memory:").await.unwrap_or_else(|e| panic!("open: {e}"));
        TursoUserStore::new(db)
    }

    fn make_new_user(email: &str) -> NewUser {
        NewUser {
            email: email.to_string(),
            name: Some("Test User".to_string()),
            avatar_url: None,
        }
    }

    #[tokio::test]
    async fn create_and_find_by_id() {
        let store = store().await;
        let user = store.create(make_new_user("a@test.com")).await.unwrap();

        let found = store.find_by_id(user.id).await.unwrap();
        assert_eq!(found.unwrap().email, "a@test.com");
    }

    #[tokio::test]
    async fn find_by_id_missing_returns_none() {
        let store = store().await;
        let found = store.find_by_id(Uuid::now_v7()).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn find_by_email() {
        let store = store().await;
        store.create(make_new_user("b@test.com")).await.unwrap();

        let found = store.find_by_email("b@test.com").await.unwrap();
        assert!(found.is_some());

        let missing = store.find_by_email("nope@test.com").await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn update_profile_changes_fields() {
        let store = store().await;
        let user = store.create(make_new_user("c@test.com")).await.unwrap();

        let updated = store
            .update_profile(user.id, Some("New Name"), Some("https://img.test/1.png"))
            .await
            .unwrap();
        assert_eq!(updated.name, Some("New Name".into()));
        assert_eq!(updated.avatar_url, Some("https://img.test/1.png".into()));
    }

    #[tokio::test]
    async fn new_user_gets_default_role() {
        let store = store().await;
        let user = store.create(make_new_user("d@test.com")).await.unwrap();
        assert_eq!(user.role, "member");
    }
}
