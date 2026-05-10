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
}
