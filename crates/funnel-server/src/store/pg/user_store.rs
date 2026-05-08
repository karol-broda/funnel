use sqlx::PgPool;
use uuid::Uuid;

use crate::db::users::{self, NewUser, User};
use crate::store::user_store::UserStore;
use crate::store::{BoxFuture, StoreError};

pub struct PgUserStore {
    pool: PgPool,
}

impl PgUserStore {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl UserStore for PgUserStore {
    fn find_by_id(&self, id: Uuid) -> BoxFuture<'_, Result<Option<User>, StoreError>> {
        Box::pin(async move {
            Ok(users::find_by_id(&self.pool, id).await?)
        })
    }

    fn find_by_email(&self, email: &str) -> BoxFuture<'_, Result<Option<User>, StoreError>> {
        let email = email.to_string();
        Box::pin(async move {
            Ok(users::find_by_email(&self.pool, &email).await?)
        })
    }

    fn find_by_provider(&self, provider: &str, provider_id: &str) -> BoxFuture<'_, Result<Option<User>, StoreError>> {
        let provider = provider.to_string();
        let provider_id = provider_id.to_string();
        Box::pin(async move {
            Ok(users::find_by_provider(&self.pool, &provider, &provider_id).await?)
        })
    }

    fn create(&self, new_user: NewUser) -> BoxFuture<'_, Result<User, StoreError>> {
        Box::pin(async move {
            Ok(users::create(&self.pool, new_user).await?)
        })
    }

    fn upsert_from_oauth(&self, new_user: NewUser) -> BoxFuture<'_, Result<User, StoreError>> {
        Box::pin(async move {
            Ok(users::upsert_from_oauth(&self.pool, new_user).await?)
        })
    }
}
