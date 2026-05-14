use uuid::Uuid;

use super::StoreError;
use crate::db::users::{NewUser, Role, User};

#[async_trait::async_trait]
pub trait UserStore: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, StoreError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, StoreError>;
    async fn create(&self, new_user: NewUser) -> Result<User, StoreError>;
    async fn update_profile(
        &self,
        id: Uuid,
        name: Option<&str>,
        avatar_url: Option<&str>,
    ) -> Result<User, StoreError>;
    async fn update_role(&self, id: Uuid, role: Role) -> Result<User, StoreError>;
    async fn deactivate(&self, id: Uuid) -> Result<User, StoreError>;
    async fn reactivate(&self, id: Uuid) -> Result<User, StoreError>;
    async fn list_all(&self, limit: i64) -> Result<Vec<User>, StoreError>;
    async fn count(&self) -> Result<i64, StoreError>;
    async fn count_admins(&self) -> Result<i64, StoreError>;
}
