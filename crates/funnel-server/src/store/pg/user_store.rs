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
        Box::pin(async move { Ok(users::find_by_id(&self.pool, id).await?) })
    }

    fn find_by_email(&self, email: &str) -> BoxFuture<'_, Result<Option<User>, StoreError>> {
        let email = email.to_string();
        Box::pin(async move { Ok(users::find_by_email(&self.pool, &email).await?) })
    }

    fn create(&self, new_user: NewUser) -> BoxFuture<'_, Result<User, StoreError>> {
        Box::pin(async move { Ok(users::create(&self.pool, new_user).await?) })
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
            Ok(
                users::update_profile(&self.pool, id, name.as_deref(), avatar_url.as_deref())
                    .await?,
            )
        })
    }
}
