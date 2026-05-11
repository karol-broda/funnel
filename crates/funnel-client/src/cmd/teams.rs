use funnel_core::protocol::PROTOCOL_VERSION;
use serde::{Deserialize, Serialize};

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

pub async fn run(server: &str, token: &str, command: Command) -> anyhow::Result<()> {
    match command {
        Command::List => list(server, token).await,
        Command::Create { name } => create(server, token, &name).await,
        Command::Delete { id } => delete(server, token, &id).await,
        Command::Members { id } => members(server, token, &id).await,
        Command::AddMember { team_id, user_id } => {
            add_member(server, token, &team_id, &user_id).await
        }
        Command::RemoveMember { team_id, user_id } => {
            remove_member(server, token, &team_id, &user_id).await
        }
        Command::SetRole {
            team_id,
            user_id,
            role,
        } => set_role(server, token, &team_id, &user_id, &role).await,
    }
}

#[derive(Deserialize)]
struct Team {
    id: String,
    name: String,
}

#[derive(Deserialize)]
struct TeamMembership {
    user_id: String,
    role: String,
    created_at: String,
}

fn client(token: &str) -> reqwest::Client {
    reqwest::Client::builder()
        .default_headers({
            let mut h = reqwest::header::HeaderMap::new();
            if let Ok(v) = reqwest::header::HeaderValue::from_str(&format!("Bearer {token}")) {
                h.insert(reqwest::header::AUTHORIZATION, v);
            }
            h
        })
        .build()
        .unwrap_or_default()
}

pub async fn list(server: &str, token: &str) -> anyhow::Result<()> {
    let resp = client(token)
        .get(format!("{server}/api/v{PROTOCOL_VERSION}/teams"))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("server returned {status}: {body}");
    }

    let teams: Vec<Team> = resp.json().await?;

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

#[derive(Serialize)]
struct CreateTeamRequest {
    name: String,
}

pub async fn create(server: &str, token: &str, name: &str) -> anyhow::Result<()> {
    let resp = client(token)
        .post(format!("{server}/api/v{PROTOCOL_VERSION}/teams"))
        .json(&CreateTeamRequest {
            name: name.to_string(),
        })
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("server returned {status}: {body}");
    }

    let team: Team = resp.json().await?;
    println!("created team '{}' ({})", team.name, team.id);
    Ok(())
}

pub async fn delete(server: &str, token: &str, id: &str) -> anyhow::Result<()> {
    let resp = client(token)
        .delete(format!("{server}/api/v{PROTOCOL_VERSION}/teams/{id}"))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("server returned {status}: {body}");
    }

    println!("deleted team {id}");
    Ok(())
}

pub async fn members(server: &str, token: &str, id: &str) -> anyhow::Result<()> {
    let resp = client(token)
        .get(format!("{server}/api/v{PROTOCOL_VERSION}/teams/{id}/members"))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("server returned {status}: {body}");
    }

    let members: Vec<TeamMembership> = resp.json().await?;

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

fn format_timestamp(ts: &str) -> &str {
    let without_frac = ts.split('.').next().unwrap_or(ts);
    without_frac.strip_suffix('Z').unwrap_or(without_frac)
}

#[derive(Serialize)]
struct AddMemberRequest {
    user_id: String,
}

pub async fn add_member(server: &str, token: &str, team_id: &str, user_id: &str) -> anyhow::Result<()> {
    let resp = client(token)
        .post(format!("{server}/api/v{PROTOCOL_VERSION}/teams/{team_id}/members"))
        .json(&AddMemberRequest {
            user_id: user_id.to_string(),
        })
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("server returned {status}: {body}");
    }

    println!("added user {user_id} to team {team_id}");
    Ok(())
}

pub async fn remove_member(server: &str, token: &str, team_id: &str, user_id: &str) -> anyhow::Result<()> {
    let resp = client(token)
        .delete(format!("{server}/api/v{PROTOCOL_VERSION}/teams/{team_id}/members/{user_id}"))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("server returned {status}: {body}");
    }

    println!("removed user {user_id} from team {team_id}");
    Ok(())
}

#[derive(Serialize)]
struct SetRoleRequest {
    role: String,
}

pub async fn set_role(
    server: &str,
    token: &str,
    team_id: &str,
    user_id: &str,
    role: &str,
) -> anyhow::Result<()> {
    let resp = client(token)
        .put(format!(
            "{server}/api/v{PROTOCOL_VERSION}/teams/{team_id}/members/{user_id}/role"
        ))
        .json(&SetRoleRequest {
            role: role.to_string(),
        })
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("server returned {status}: {body}");
    }

    println!("set role '{role}' for user {user_id} in team {team_id}");
    Ok(())
}
