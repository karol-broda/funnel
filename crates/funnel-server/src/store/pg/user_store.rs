use sqlx::PgPool;
use uuid::Uuid;

use crate::db::users::{self, NewUser, Role, User};
use crate::store::StoreError;
use crate::store::user_store::UserStore;

pub struct PgUserStore {
    pool: PgPool,
}

impl PgUserStore {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl UserStore for PgUserStore {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, StoreError> {
        Ok(users::find_by_id(&self.pool, id).await?)
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, StoreError> {
        Ok(users::find_by_email(&self.pool, email).await?)
    }

    async fn create(&self, new_user: NewUser) -> Result<User, StoreError> {
        Ok(users::create(&self.pool, new_user).await?)
    }

    async fn update_profile(
        &self,
        id: Uuid,
        name: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<User, StoreError> {
        Ok(users::update_profile(&self.pool, id, name, avatar_url).await?)
    }

    async fn update_role(&self, id: Uuid, role: Role) -> Result<User, StoreError> {
        Ok(users::update_role(&self.pool, id, role).await?)
    }

    async fn deactivate(&self, id: Uuid) -> Result<User, StoreError> {
        Ok(users::deactivate(&self.pool, id).await?)
    }

    async fn reactivate(&self, id: Uuid) -> Result<User, StoreError> {
        Ok(users::reactivate(&self.pool, id).await?)
    }

    async fn list_all(&self, limit: i64) -> Result<Vec<User>, StoreError> {
        Ok(users::list_all(&self.pool, limit).await?)
    }

    async fn count(&self) -> Result<i64, StoreError> {
        Ok(users::count(&self.pool).await?)
    }

    async fn count_admins(&self) -> Result<i64, StoreError> {
        Ok(users::count_admins(&self.pool).await?)
    }
}
