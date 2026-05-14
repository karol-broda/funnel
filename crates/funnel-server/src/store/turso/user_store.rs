use std::sync::Arc;

use chrono::Utc;
use turso::Database;
use uuid::Uuid;

use super::{format_dt, map_err, parse_dt, parse_optional_dt, parse_uuid};
use crate::db::users::{NewUser, User};
use crate::store::user_store::UserStore;
use crate::store::StoreError;
use funnel_core::api::Role;

pub struct TursoUserStore {
    db: Arc<Database>,
}

impl TursoUserStore {
    pub const fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

fn row_to_user(row: &turso::Row) -> Result<User, StoreError> {
    Ok(User {
        id: parse_uuid(&row.get::<String>(0).map_err(|e| map_err(&e))?)?,
        email: row.get::<String>(1).map_err(|e| map_err(&e))?,
        name: row.get::<Option<String>>(2).map_err(|e| map_err(&e))?,
        avatar_url: row.get::<Option<String>>(3).map_err(|e| map_err(&e))?,
        role: serde_json::from_value(serde_json::Value::String(
            row.get::<String>(4).map_err(|e| map_err(&e))?,
        ))
        .map_err(|e| StoreError::Other(format!("invalid role: {e}")))?,
        metadata: serde_json::from_str(&row.get::<String>(5).map_err(|e| map_err(&e))?)
            .map_err(|e| StoreError::Other(format!("invalid json: {e}")))?,
        created_at: parse_dt(&row.get::<String>(6).map_err(|e| map_err(&e))?)?,
        updated_at: parse_dt(&row.get::<String>(7).map_err(|e| map_err(&e))?)?,
        deactivated_at: parse_optional_dt(row.get::<Option<String>>(8).map_err(|e| map_err(&e))?)?,
    })
}

#[async_trait::async_trait]
impl UserStore for TursoUserStore {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, StoreError> {
        let conn = self.db.connect().map_err(|e| map_err(&e))?;
        let mut rows = conn
            .query(
                "SELECT id, email, name, avatar_url, role, metadata, created_at, updated_at, deactivated_at FROM users WHERE id = ?",
                turso::params![id.to_string()],
            )
            .await
            .map_err(|e| map_err(&e))?;
        match rows.next().await.map_err(|e| map_err(&e))? {
            Some(row) => Ok(Some(row_to_user(&row)?)),
            None => Ok(None),
        }
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, StoreError> {
        let conn = self.db.connect().map_err(|e| map_err(&e))?;
        let mut rows = conn
            .query(
                "SELECT id, email, name, avatar_url, role, metadata, created_at, updated_at, deactivated_at FROM users WHERE email = ?",
                turso::params![email.to_string()],
            )
            .await
            .map_err(|e| map_err(&e))?;
        match rows.next().await.map_err(|e| map_err(&e))? {
            Some(row) => Ok(Some(row_to_user(&row)?)),
            None => Ok(None),
        }
    }

    async fn create(&self, new_user: NewUser) -> Result<User, StoreError> {
        let conn = self.db.connect().map_err(|e| map_err(&e))?;
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
                Role::Member.as_str(),
                metadata,
                now_str.clone(),
                now_str
            ],
        )
        .await
        .map_err(|e| map_err(&e))?;
        Ok(User {
            id,
            email: new_user.email,
            name: new_user.name,
            avatar_url: new_user.avatar_url,
            role: Role::Member,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            deactivated_at: None,
        })
    }

    async fn update_profile(
        &self,
        id: Uuid,
        name: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<User, StoreError> {
        let conn = self.db.connect().map_err(|e| map_err(&e))?;
        let now = format_dt(Utc::now());
        conn.execute(
            "UPDATE users SET name = ?, avatar_url = ?, updated_at = ? WHERE id = ?",
            turso::params![
                name.unwrap_or_default(),
                avatar_url.unwrap_or_default(),
                now,
                id.to_string()
            ],
        )
        .await
        .map_err(|e| map_err(&e))?;
        let user = self.find_by_id(id).await?.ok_or(StoreError::NotFound)?;
        Ok(user)
    }

    async fn update_role(&self, id: Uuid, role: Role) -> Result<User, StoreError> {
        let conn = self.db.connect().map_err(|e| map_err(&e))?;
        let now = format_dt(Utc::now());
        conn.execute(
            "UPDATE users SET role = ?, updated_at = ? WHERE id = ?",
            turso::params![role.as_str(), now, id.to_string()],
        )
        .await
        .map_err(|e| map_err(&e))?;
        let user = self.find_by_id(id).await?.ok_or(StoreError::NotFound)?;
        Ok(user)
    }

    async fn deactivate(&self, id: Uuid) -> Result<User, StoreError> {
        let conn = self.db.connect().map_err(|e| map_err(&e))?;
        let now = format_dt(Utc::now());
        conn.execute(
            "UPDATE users SET deactivated_at = ?, updated_at = ? WHERE id = ?",
            turso::params![now.clone(), now, id.to_string()],
        )
        .await
        .map_err(|e| map_err(&e))?;
        let user = self.find_by_id(id).await?.ok_or(StoreError::NotFound)?;
        Ok(user)
    }

    async fn reactivate(&self, id: Uuid) -> Result<User, StoreError> {
        let conn = self.db.connect().map_err(|e| map_err(&e))?;
        let now = format_dt(Utc::now());
        conn.execute(
            "UPDATE users SET deactivated_at = NULL, updated_at = ? WHERE id = ?",
            turso::params![now, id.to_string()],
        )
        .await
        .map_err(|e| map_err(&e))?;
        let user = self.find_by_id(id).await?.ok_or(StoreError::NotFound)?;
        Ok(user)
    }

    async fn list_all(&self, limit: i64) -> Result<Vec<User>, StoreError> {
        let conn = self.db.connect().map_err(|e| map_err(&e))?;
        let mut rows = conn
            .query(
                "SELECT id, email, name, avatar_url, role, metadata, created_at, updated_at, deactivated_at FROM users ORDER BY created_at DESC LIMIT ?",
                turso::params![limit],
            )
            .await
            .map_err(|e| map_err(&e))?;
        let mut users = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| map_err(&e))? {
            users.push(row_to_user(&row)?);
        }
        Ok(users)
    }

    async fn count(&self) -> Result<i64, StoreError> {
        let conn = self.db.connect().map_err(|e| map_err(&e))?;
        let mut rows = conn
            .query("SELECT COUNT(*) FROM users", ())
            .await
            .map_err(|e| map_err(&e))?;
        let row = rows
            .next()
            .await
            .map_err(|e| map_err(&e))?
            .ok_or(StoreError::Other("no count row".into()))?;
        row.get::<i64>(0).map_err(|e| map_err(&e))
    }

    async fn count_admins(&self) -> Result<i64, StoreError> {
        let conn = self.db.connect().map_err(|e| map_err(&e))?;
        let mut rows = conn
            .query(
                "SELECT COUNT(*) FROM users WHERE role = 'admin' AND deactivated_at IS NULL",
                (),
            )
            .await
            .map_err(|e| map_err(&e))?;
        let row = rows
            .next()
            .await
            .map_err(|e| map_err(&e))?
            .ok_or(StoreError::Other("no count row".into()))?;
        row.get::<i64>(0).map_err(|e| map_err(&e))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::store::turso::open;

    async fn store() -> TursoUserStore {
        let db = open(":memory:")
            .await
            .unwrap_or_else(|e| panic!("open: {e}"));
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
        assert_eq!(user.role, Role::Member);
    }
}
