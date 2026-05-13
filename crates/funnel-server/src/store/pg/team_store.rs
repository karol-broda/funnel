use sqlx::PgPool;
use uuid::Uuid;

use crate::db::teams::{self, Team, TeamMembership};
use crate::store::team_store::TeamStore;
use crate::store::{BoxFuture, StoreError};

pub struct PgTeamStore {
    pool: PgPool,
}

impl PgTeamStore {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl TeamStore for PgTeamStore {
    fn create(&self, name: &str) -> BoxFuture<'_, Result<Team, StoreError>> {
        let name = name.to_string();
        Box::pin(async move { Ok(teams::create(&self.pool, &name).await?) })
    }

    fn find_by_id(&self, id: Uuid) -> BoxFuture<'_, Result<Option<Team>, StoreError>> {
        Box::pin(async move { Ok(teams::find_by_id(&self.pool, id).await?) })
    }

    fn find_by_name(&self, name: &str) -> BoxFuture<'_, Result<Option<Team>, StoreError>> {
        let name = name.to_string();
        Box::pin(async move { Ok(teams::find_by_name(&self.pool, &name).await?) })
    }

    fn list_all(&self) -> BoxFuture<'_, Result<Vec<Team>, StoreError>> {
        Box::pin(async move { Ok(teams::list_all(&self.pool).await?) })
    }

    fn delete(&self, id: Uuid) -> BoxFuture<'_, Result<bool, StoreError>> {
        Box::pin(async move { Ok(teams::delete(&self.pool, id).await?) })
    }

    fn add_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> BoxFuture<'_, Result<TeamMembership, StoreError>> {
        let role = role.to_string();
        Box::pin(async move { Ok(teams::add_member(&self.pool, team_id, user_id, &role).await?) })
    }

    fn remove_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> BoxFuture<'_, Result<bool, StoreError>> {
        Box::pin(async move { Ok(teams::remove_member(&self.pool, team_id, user_id).await?) })
    }

    fn list_members(
        &self,
        team_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<TeamMembership>, StoreError>> {
        Box::pin(async move { Ok(teams::list_members(&self.pool, team_id).await?) })
    }

    fn list_teams_for_user(&self, user_id: Uuid) -> BoxFuture<'_, Result<Vec<Team>, StoreError>> {
        Box::pin(async move { Ok(teams::list_teams_for_user(&self.pool, user_id).await?) })
    }

    fn get_team_ids_for_user(&self, user_id: Uuid) -> BoxFuture<'_, Result<Vec<Uuid>, StoreError>> {
        Box::pin(async move { Ok(teams::get_team_ids_for_user(&self.pool, user_id).await?) })
    }

    fn is_member(&self, team_id: Uuid, user_id: Uuid) -> BoxFuture<'_, Result<bool, StoreError>> {
        Box::pin(async move { Ok(teams::is_member(&self.pool, team_id, user_id).await?) })
    }

    fn update_member_role(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> BoxFuture<'_, Result<TeamMembership, StoreError>> {
        let role = role.to_string();
        Box::pin(async move {
            Ok(teams::update_member_role(&self.pool, team_id, user_id, &role).await?)
        })
    }

    fn find_membership(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> BoxFuture<'_, Result<Option<TeamMembership>, StoreError>> {
        Box::pin(async move { Ok(teams::find_membership(&self.pool, team_id, user_id).await?) })
    }

    fn count_owners(&self, team_id: Uuid) -> BoxFuture<'_, Result<i64, StoreError>> {
        Box::pin(async move { Ok(teams::count_owners(&self.pool, team_id).await?) })
    }
}
