use funnel_core::api::{
    self as endpoints, AddMemberRequest, CreateTeamRequest, SetMemberRoleRequest, TeamRole,
};

use super::api;

fn parse_team_role(input: &str) -> anyhow::Result<TeamRole> {
    serde_json::from_value(serde_json::Value::String(input.to_string()))
        .map_err(|_| anyhow::anyhow!("invalid role '{input}', must be 'owner' or 'member'"))
}

#[derive(clap::Subcommand)]
pub enum Command {
    /// list teams
    List,
    /// create a new team
    Create {
        /// team name
        name: String,
    },
    /// delete a team
    Delete {
        /// team id
        id: String,
    },
    /// list team members
    Members {
        /// team id
        id: String,
    },
    /// add a member to a team
    AddMember {
        /// team id
        team_id: String,
        /// user id to add
        user_id: String,
    },
    /// remove a member from a team
    RemoveMember {
        /// team id
        team_id: String,
        /// user id to remove
        user_id: String,
    },
    /// set a member's role in a team
    SetRole {
        /// team id
        team_id: String,
        /// user id
        user_id: String,
        /// role (owner or member)
        role: String,
    },
}

pub async fn run(server: &str, token: &str, command: Command, json: bool) -> anyhow::Result<()> {
    match command {
        Command::List => list(server, token, json).await,
        Command::Create { name } => create(server, token, &name, json).await,
        Command::Delete { id } => delete(server, token, &id, json).await,
        Command::Members { id } => members(server, token, &id, json).await,
        Command::AddMember { team_id, user_id } => {
            add_member(server, token, &team_id, &user_id, json).await
        }
        Command::RemoveMember { team_id, user_id } => {
            remove_member(server, token, &team_id, &user_id, json).await
        }
        Command::SetRole {
            team_id,
            user_id,
            role,
        } => set_role(server, token, &team_id, &user_id, &role, json).await,
    }
}

fn format_timestamp(ts: &chrono::DateTime<chrono::Utc>) -> String {
    ts.format("%Y-%m-%dT%H:%M:%S").to_string()
}

pub async fn list(server: &str, token: &str, json: bool) -> anyhow::Result<()> {
    let Some(teams) = api::call(server, token, &endpoints::TEAMS_LIST, json).await? else {
        return Ok(());
    };

    if teams.is_empty() {
        println!("no teams");
        return Ok(());
    }

    for (i, t) in teams.iter().enumerate() {
        println!("{}", t.name);
        println!("  id: {}", t.id);
        if i < teams.len() - 1 {
            println!();
        }
    }

    if teams.len() > 1 {
        println!("\n{} teams", teams.len());
    }

    Ok(())
}

pub async fn create(server: &str, token: &str, name: &str, json: bool) -> anyhow::Result<()> {
    let body = CreateTeamRequest {
        name: name.to_string(),
        owner_id: None,
    };
    let Some(team) = api::send(server, token, &endpoints::TEAMS_CREATE, &body, json).await? else {
        return Ok(());
    };
    println!("created team '{}' ({})", team.name, team.id);
    Ok(())
}

pub async fn delete(server: &str, token: &str, id: &str, json: bool) -> anyhow::Result<()> {
    let Some(_) = api::call_at(
        server,
        token,
        &endpoints::TEAMS_DELETE,
        &format!("/teams/{id}"),
        json,
    )
    .await?
    else {
        return Ok(());
    };

    println!("deleted team {id}");
    Ok(())
}

pub async fn members(server: &str, token: &str, id: &str, json: bool) -> anyhow::Result<()> {
    let Some(members) = api::call_at(
        server,
        token,
        &endpoints::TEAMS_MEMBERS,
        &format!("/teams/{id}/members"),
        json,
    )
    .await?
    else {
        return Ok(());
    };

    if members.is_empty() {
        println!("no members");
        return Ok(());
    }

    for m in &members {
        println!("{} ({})", m.user_id, m.role);
        println!("  joined: {}", format_timestamp(&m.created_at));
    }

    if members.len() > 1 {
        println!("\n{} members", members.len());
    }

    Ok(())
}

pub async fn add_member(
    server: &str,
    token: &str,
    team_id: &str,
    user_id: &str,
    json: bool,
) -> anyhow::Result<()> {
    let parsed_user_id: uuid::Uuid = user_id.parse()?;
    let body = AddMemberRequest {
        user_id: parsed_user_id,
    };
    let Some(_) = api::send_at(
        server,
        token,
        &endpoints::TEAMS_ADD_MEMBER,
        &format!("/teams/{team_id}/members"),
        &body,
        json,
    )
    .await?
    else {
        return Ok(());
    };

    println!("added user {user_id} to team {team_id}");
    Ok(())
}

pub async fn remove_member(
    server: &str,
    token: &str,
    team_id: &str,
    user_id: &str,
    json: bool,
) -> anyhow::Result<()> {
    let Some(_) = api::call_at(
        server,
        token,
        &endpoints::TEAMS_REMOVE_MEMBER,
        &format!("/teams/{team_id}/members/{user_id}"),
        json,
    )
    .await?
    else {
        return Ok(());
    };

    println!("removed user {user_id} from team {team_id}");
    Ok(())
}

pub async fn set_role(
    server: &str,
    token: &str,
    team_id: &str,
    user_id: &str,
    role: &str,
    json: bool,
) -> anyhow::Result<()> {
    let role: TeamRole = parse_team_role(role)?;
    let body = SetMemberRoleRequest { role };
    let Some(_) = api::send_at(
        server,
        token,
        &endpoints::TEAMS_SET_MEMBER_ROLE,
        &format!("/teams/{team_id}/members/{user_id}/role"),
        &body,
        json,
    )
    .await?
    else {
        return Ok(());
    };

    println!("set role '{role}' for user {user_id} in team {team_id}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_team_role_accepts_owner() {
        assert_eq!(parse_team_role("owner").unwrap(), TeamRole::Owner);
    }

    #[test]
    fn parse_team_role_accepts_member() {
        assert_eq!(parse_team_role("member").unwrap(), TeamRole::Member);
    }

    #[test]
    fn parse_team_role_rejects_invalid() {
        assert!(parse_team_role("admin").is_err());
        assert!(parse_team_role("manager").is_err());
    }

    #[test]
    fn parse_team_role_rejects_empty() {
        assert!(parse_team_role("").is_err());
    }

    #[test]
    fn parse_team_role_rejects_wrong_case() {
        assert!(parse_team_role("Owner").is_err());
        assert!(parse_team_role("MEMBER").is_err());
    }

    #[test]
    fn parse_team_role_error_message_includes_input() {
        let err = parse_team_role("admin").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("admin"), "error should include input: {msg}");
        assert!(msg.contains("owner"), "error should hint valid values: {msg}");
    }
}
