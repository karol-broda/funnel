use uuid::Uuid;

use super::{BoxFuture, StoreError};
use crate::db::users::{NewUser, User};

pub trait UserStore: Send + Sync {
    fn find_by_id(&self, id: Uuid) -> BoxFuture<'_, Result<Option<User>, StoreError>>;
    fn find_by_email(&self, email: &str) -> BoxFuture<'_, Result<Option<User>, StoreError>>;
    fn create(&self, new_user: NewUser) -> BoxFuture<'_, Result<User, StoreError>>;
    fn update_profile(
        &self,
        id: Uuid,
        name: Option<&str>,
        avatar_url: Option<&str>,
    ) -> BoxFuture<'_, Result<User, StoreError>>;
    fn update_role(&self, id: Uuid, role: &str) -> BoxFuture<'_, Result<User, StoreError>>;
    fn deactivate(&self, id: Uuid) -> BoxFuture<'_, Result<User, StoreError>>;
    fn reactivate(&self, id: Uuid) -> BoxFuture<'_, Result<User, StoreError>>;
    fn list_all(&self, limit: i64) -> BoxFuture<'_, Result<Vec<User>, StoreError>>;
    fn count(&self) -> BoxFuture<'_, Result<i64, StoreError>>;
    fn count_admins(&self) -> BoxFuture<'_, Result<i64, StoreError>>;
}
