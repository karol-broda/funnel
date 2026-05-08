use uuid::Uuid;

use crate::db::users::{NewUser, User};
use super::{BoxFuture, StoreError};

pub trait UserStore: Send + Sync {
    fn find_by_id(&self, id: Uuid) -> BoxFuture<'_, Result<Option<User>, StoreError>>;
    fn find_by_email(&self, email: &str) -> BoxFuture<'_, Result<Option<User>, StoreError>>;
    fn find_by_provider(&self, provider: &str, provider_id: &str) -> BoxFuture<'_, Result<Option<User>, StoreError>>;
    fn create(&self, new_user: NewUser) -> BoxFuture<'_, Result<User, StoreError>>;
    fn upsert_from_oauth(&self, new_user: NewUser) -> BoxFuture<'_, Result<User, StoreError>>;
}
