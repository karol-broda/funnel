use uuid::Uuid;

use super::{BoxFuture, StoreError};
use crate::db::teams::{Team, TeamMembership};

pub trait TeamStore: Send + Sync {
    fn create(&self, name: &str) -> BoxFuture<'_, Result<Team, StoreError>>;
    fn find_by_id(&self, id: Uuid) -> BoxFuture<'_, Result<Option<Team>, StoreError>>;
    fn find_by_name(&self, name: &str) -> BoxFuture<'_, Result<Option<Team>, StoreError>>;
    fn list_all(&self) -> BoxFuture<'_, Result<Vec<Team>, StoreError>>;
    fn delete(&self, id: Uuid) -> BoxFuture<'_, Result<bool, StoreError>>;
    fn add_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> BoxFuture<'_, Result<TeamMembership, StoreError>>;
    fn remove_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> BoxFuture<'_, Result<bool, StoreError>>;
    fn list_members(
        &self,
        team_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<TeamMembership>, StoreError>>;
    fn list_teams_for_user(&self, user_id: Uuid) -> BoxFuture<'_, Result<Vec<Team>, StoreError>>;
    fn get_team_ids_for_user(&self, user_id: Uuid) -> BoxFuture<'_, Result<Vec<Uuid>, StoreError>>;
    fn is_member(&self, team_id: Uuid, user_id: Uuid) -> BoxFuture<'_, Result<bool, StoreError>>;
    fn update_member_role(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> BoxFuture<'_, Result<TeamMembership, StoreError>>;
    fn find_membership(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> BoxFuture<'_, Result<Option<TeamMembership>, StoreError>>;
    fn count_owners(&self, team_id: Uuid) -> BoxFuture<'_, Result<i64, StoreError>>;
}
