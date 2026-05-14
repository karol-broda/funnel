use sqlx::PgPool;
use uuid::Uuid;

use crate::db::teams::{self, Team, TeamMembership, TeamRole};
use crate::store::team_store::TeamStore;
use crate::store::StoreError;

pub struct PgTeamStore {
    pool: PgPool,
}

impl PgTeamStore {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl TeamStore for PgTeamStore {
    async fn create(&self, name: &str) -> Result<Team, StoreError> {
        Ok(teams::create(&self.pool, name).await?)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Team>, StoreError> {
        Ok(teams::find_by_id(&self.pool, id).await?)
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Team>, StoreError> {
        Ok(teams::find_by_name(&self.pool, name).await?)
    }

    async fn list_all(&self) -> Result<Vec<Team>, StoreError> {
        Ok(teams::list_all(&self.pool).await?)
    }

    async fn delete(&self, id: Uuid) -> Result<bool, StoreError> {
        Ok(teams::delete(&self.pool, id).await?)
    }

    async fn add_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: TeamRole,
    ) -> Result<TeamMembership, StoreError> {
        Ok(teams::add_member(&self.pool, team_id, user_id, role).await?)
    }

    async fn remove_member(&self, team_id: Uuid, user_id: Uuid) -> Result<bool, StoreError> {
        Ok(teams::remove_member(&self.pool, team_id, user_id).await?)
    }

    async fn list_members(&self, team_id: Uuid) -> Result<Vec<TeamMembership>, StoreError> {
        Ok(teams::list_members(&self.pool, team_id).await?)
    }

    async fn list_teams_for_user(&self, user_id: Uuid) -> Result<Vec<Team>, StoreError> {
        Ok(teams::list_teams_for_user(&self.pool, user_id).await?)
    }

    async fn get_team_ids_for_user(&self, user_id: Uuid) -> Result<Vec<Uuid>, StoreError> {
        Ok(teams::get_team_ids_for_user(&self.pool, user_id).await?)
    }

    async fn is_member(&self, team_id: Uuid, user_id: Uuid) -> Result<bool, StoreError> {
        Ok(teams::is_member(&self.pool, team_id, user_id).await?)
    }

    async fn update_member_role(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: TeamRole,
    ) -> Result<TeamMembership, StoreError> {
        Ok(teams::update_member_role(&self.pool, team_id, user_id, role).await?)
    }

    async fn find_membership(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<TeamMembership>, StoreError> {
        Ok(teams::find_membership(&self.pool, team_id, user_id).await?)
    }

    async fn count_owners(&self, team_id: Uuid) -> Result<i64, StoreError> {
        Ok(teams::count_owners(&self.pool, team_id).await?)
    }
}
