use uuid::Uuid;

use super::StoreError;
use crate::db::teams::{Team, TeamMembership, TeamRole};

#[async_trait::async_trait]
pub trait TeamStore: Send + Sync {
    async fn create(&self, name: &str) -> Result<Team, StoreError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Team>, StoreError>;
    async fn find_by_name(&self, name: &str) -> Result<Option<Team>, StoreError>;
    async fn list_all(&self) -> Result<Vec<Team>, StoreError>;
    async fn delete(&self, id: Uuid) -> Result<bool, StoreError>;
    async fn add_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: TeamRole,
    ) -> Result<TeamMembership, StoreError>;
    async fn remove_member(&self, team_id: Uuid, user_id: Uuid) -> Result<bool, StoreError>;
    async fn list_members(&self, team_id: Uuid) -> Result<Vec<TeamMembership>, StoreError>;
    async fn list_teams_for_user(&self, user_id: Uuid) -> Result<Vec<Team>, StoreError>;
    async fn get_team_ids_for_user(&self, user_id: Uuid) -> Result<Vec<Uuid>, StoreError>;
    async fn is_member(&self, team_id: Uuid, user_id: Uuid) -> Result<bool, StoreError>;
    async fn update_member_role(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: TeamRole,
    ) -> Result<TeamMembership, StoreError>;
    async fn find_membership(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<TeamMembership>, StoreError>;
    async fn count_owners(&self, team_id: Uuid) -> Result<i64, StoreError>;
}
