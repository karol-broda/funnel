use std::sync::Arc;

use chrono::Utc;
use turso::Database;
use uuid::Uuid;

use super::{format_dt, map_err, parse_dt, parse_uuid};
use crate::db::teams::{Team, TeamMembership};
use crate::store::team_store::TeamStore;
use crate::store::{BoxFuture, StoreError};

pub struct TursoTeamStore {
    db: Arc<Database>,
}

impl TursoTeamStore {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }
}

fn row_to_team(row: &turso::Row) -> Result<Team, StoreError> {
    Ok(Team {
        id: parse_uuid(&row.get::<String>(0).map_err(map_err)?)?,
        name: row.get::<String>(1).map_err(map_err)?,
        created_at: parse_dt(&row.get::<String>(2).map_err(map_err)?)?,
        updated_at: parse_dt(&row.get::<String>(3).map_err(map_err)?)?,
    })
}

fn row_to_membership(row: &turso::Row) -> Result<TeamMembership, StoreError> {
    Ok(TeamMembership {
        id: parse_uuid(&row.get::<String>(0).map_err(map_err)?)?,
        team_id: parse_uuid(&row.get::<String>(1).map_err(map_err)?)?,
        user_id: parse_uuid(&row.get::<String>(2).map_err(map_err)?)?,
        role: row.get::<String>(3).map_err(map_err)?,
        created_at: parse_dt(&row.get::<String>(4).map_err(map_err)?)?,
    })
}

impl TeamStore for TursoTeamStore {
    fn create(&self, name: &str) -> BoxFuture<'_, Result<Team, StoreError>> {
        let name = name.to_string();
        Box::pin(async move {
            let conn = self.db.connect().map_err(map_err)?;
            let id = Uuid::now_v7();
            let now = Utc::now();
            let now_str = format_dt(now);
            conn.execute(
                "INSERT INTO teams (id, name, created_at, updated_at) VALUES (?, ?, ?, ?)",
                turso::params![id.to_string(), name.clone(), now_str.clone(), now_str],
            )
            .await
            .map_err(map_err)?;
            Ok(Team {
                id,
                name,
                created_at: now,
                updated_at: now,
            })
        })
    }

    fn find_by_id(&self, id: Uuid) -> BoxFuture<'_, Result<Option<Team>, StoreError>> {
        Box::pin(async move {
            let conn = self.db.connect().map_err(map_err)?;
            let mut rows = conn
                .query(
                    "SELECT id, name, created_at, updated_at FROM teams WHERE id = ?",
                    turso::params![id.to_string()],
                )
                .await
                .map_err(map_err)?;
            match rows.next().await.map_err(map_err)? {
                Some(row) => Ok(Some(row_to_team(&row)?)),
                None => Ok(None),
            }
        })
    }

    fn find_by_name(&self, name: &str) -> BoxFuture<'_, Result<Option<Team>, StoreError>> {
        let name = name.to_string();
        Box::pin(async move {
            let conn = self.db.connect().map_err(map_err)?;
            let mut rows = conn
                .query(
                    "SELECT id, name, created_at, updated_at FROM teams WHERE name = ?",
                    turso::params![name],
                )
                .await
                .map_err(map_err)?;
            match rows.next().await.map_err(map_err)? {
                Some(row) => Ok(Some(row_to_team(&row)?)),
                None => Ok(None),
            }
        })
    }

    fn list_all(&self) -> BoxFuture<'_, Result<Vec<Team>, StoreError>> {
        Box::pin(async move {
            let conn = self.db.connect().map_err(map_err)?;
            let mut rows = conn
                .query(
                    "SELECT id, name, created_at, updated_at FROM teams ORDER BY name",
                    (),
                )
                .await
                .map_err(map_err)?;
            let mut teams = Vec::new();
            while let Some(row) = rows.next().await.map_err(map_err)? {
                teams.push(row_to_team(&row)?);
            }
            Ok(teams)
        })
    }

    fn delete(&self, id: Uuid) -> BoxFuture<'_, Result<bool, StoreError>> {
        Box::pin(async move {
            let conn = self.db.connect().map_err(map_err)?;
            let rows_affected = conn
                .execute(
                    "DELETE FROM teams WHERE id = ?",
                    turso::params![id.to_string()],
                )
                .await
                .map_err(map_err)?;
            Ok(rows_affected > 0)
        })
    }

    fn add_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> BoxFuture<'_, Result<TeamMembership, StoreError>> {
        let role = role.to_string();
        Box::pin(async move {
            let conn = self.db.connect().map_err(map_err)?;
            let id = Uuid::now_v7();
            let now = Utc::now();
            conn.execute(
                "INSERT INTO team_memberships (id, team_id, user_id, role, created_at) VALUES (?, ?, ?, ?, ?)",
                turso::params![
                    id.to_string(),
                    team_id.to_string(),
                    user_id.to_string(),
                    role.clone(),
                    format_dt(now)
                ],
            )
            .await
            .map_err(map_err)?;
            Ok(TeamMembership {
                id,
                team_id,
                user_id,
                role,
                created_at: now,
            })
        })
    }

    fn remove_member(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> BoxFuture<'_, Result<bool, StoreError>> {
        Box::pin(async move {
            let conn = self.db.connect().map_err(map_err)?;
            let rows_affected = conn
                .execute(
                    "DELETE FROM team_memberships WHERE team_id = ? AND user_id = ?",
                    turso::params![team_id.to_string(), user_id.to_string()],
                )
                .await
                .map_err(map_err)?;
            Ok(rows_affected > 0)
        })
    }

    fn list_members(
        &self,
        team_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<TeamMembership>, StoreError>> {
        Box::pin(async move {
            let conn = self.db.connect().map_err(map_err)?;
            let mut rows = conn
                .query(
                    "SELECT id, team_id, user_id, role, created_at FROM team_memberships WHERE team_id = ? ORDER BY created_at",
                    turso::params![team_id.to_string()],
                )
                .await
                .map_err(map_err)?;
            let mut members = Vec::new();
            while let Some(row) = rows.next().await.map_err(map_err)? {
                members.push(row_to_membership(&row)?);
            }
            Ok(members)
        })
    }

    fn list_teams_for_user(&self, user_id: Uuid) -> BoxFuture<'_, Result<Vec<Team>, StoreError>> {
        Box::pin(async move {
            let conn = self.db.connect().map_err(map_err)?;
            let mut rows = conn
                .query(
                    "SELECT t.id, t.name, t.created_at, t.updated_at FROM teams t INNER JOIN team_memberships tm ON t.id = tm.team_id WHERE tm.user_id = ? ORDER BY t.name",
                    turso::params![user_id.to_string()],
                )
                .await
                .map_err(map_err)?;
            let mut teams = Vec::new();
            while let Some(row) = rows.next().await.map_err(map_err)? {
                teams.push(row_to_team(&row)?);
            }
            Ok(teams)
        })
    }

    fn get_team_ids_for_user(&self, user_id: Uuid) -> BoxFuture<'_, Result<Vec<Uuid>, StoreError>> {
        Box::pin(async move {
            let conn = self.db.connect().map_err(map_err)?;
            let mut rows = conn
                .query(
                    "SELECT team_id FROM team_memberships WHERE user_id = ?",
                    turso::params![user_id.to_string()],
                )
                .await
                .map_err(map_err)?;
            let mut ids = Vec::new();
            while let Some(row) = rows.next().await.map_err(map_err)? {
                ids.push(parse_uuid(&row.get::<String>(0).map_err(map_err)?)?);
            }
            Ok(ids)
        })
    }

    fn is_member(&self, team_id: Uuid, user_id: Uuid) -> BoxFuture<'_, Result<bool, StoreError>> {
        Box::pin(async move {
            let conn = self.db.connect().map_err(map_err)?;
            let mut rows = conn
                .query(
                    "SELECT 1 FROM team_memberships WHERE team_id = ? AND user_id = ?",
                    turso::params![team_id.to_string(), user_id.to_string()],
                )
                .await
                .map_err(map_err)?;
            Ok(rows.next().await.map_err(map_err)?.is_some())
        })
    }

    fn update_member_role(
        &self,
        team_id: Uuid,
        user_id: Uuid,
        role: &str,
    ) -> BoxFuture<'_, Result<TeamMembership, StoreError>> {
        let role = role.to_string();
        Box::pin(async move {
            let conn = self.db.connect().map_err(map_err)?;
            conn.execute(
                "UPDATE team_memberships SET role = ? WHERE team_id = ? AND user_id = ?",
                turso::params![role, team_id.to_string(), user_id.to_string()],
            )
            .await
            .map_err(map_err)?;

            let mut rows = conn
                .query(
                    "SELECT id, team_id, user_id, role, created_at FROM team_memberships WHERE team_id = ? AND user_id = ?",
                    turso::params![team_id.to_string(), user_id.to_string()],
                )
                .await
                .map_err(map_err)?;
            match rows.next().await.map_err(map_err)? {
                Some(row) => Ok(row_to_membership(&row)?),
                None => Err(StoreError::NotFound),
            }
        })
    }

    fn find_membership(
        &self,
        team_id: Uuid,
        user_id: Uuid,
    ) -> BoxFuture<'_, Result<Option<TeamMembership>, StoreError>> {
        Box::pin(async move {
            let conn = self.db.connect().map_err(map_err)?;
            let mut rows = conn
                .query(
                    "SELECT id, team_id, user_id, role, created_at FROM team_memberships WHERE team_id = ? AND user_id = ?",
                    turso::params![team_id.to_string(), user_id.to_string()],
                )
                .await
                .map_err(map_err)?;
            match rows.next().await.map_err(map_err)? {
                Some(row) => Ok(Some(row_to_membership(&row)?)),
                None => Ok(None),
            }
        })
    }

    fn count_owners(&self, team_id: Uuid) -> BoxFuture<'_, Result<i64, StoreError>> {
        Box::pin(async move {
            let conn = self.db.connect().map_err(map_err)?;
            let mut rows = conn
                .query(
                    "SELECT COUNT(*) FROM team_memberships WHERE team_id = ? AND role = 'owner'",
                    turso::params![team_id.to_string()],
                )
                .await
                .map_err(map_err)?;
            let row = rows
                .next()
                .await
                .map_err(map_err)?
                .ok_or(StoreError::Other("no count row".into()))?;
            Ok(row.get::<i64>(0).map_err(map_err)?)
        })
    }
}
