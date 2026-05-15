use funnel_core::api::{self as endpoints, Role, SetUserRoleRequest};

use super::api;

fn parse_role(input: &str) -> anyhow::Result<Role> {
    serde_json::from_value(serde_json::Value::String(input.to_string()))
        .map_err(|_| anyhow::anyhow!("invalid role '{input}', must be 'admin' or 'member'"))
}

#[derive(clap::Subcommand)]
pub enum Command {
    /// list all users
    List {
        /// maximum number of users to show
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },
    /// set a user's role
    SetRole {
        /// user id
        id: String,
        /// new role (admin or member)
        role: String,
    },
    /// deactivate a user
    Deactivate {
        /// user id
        id: String,
    },
    /// reactivate a user
    Reactivate {
        /// user id
        id: String,
    },
}

pub async fn run(server: &str, token: &str, command: Command, json: bool) -> anyhow::Result<()> {
    match command {
        Command::List { limit } => list(server, token, limit, json).await,
        Command::SetRole { id, role } => set_role(server, token, &id, &role, json).await,
        Command::Deactivate { id } => deactivate(server, token, &id, json).await,
        Command::Reactivate { id } => reactivate(server, token, &id, json).await,
    }
}

pub async fn list(server: &str, token: &str, limit: u32, json: bool) -> anyhow::Result<()> {
    let Some(users) = api::call_at(
        server,
        token,
        &endpoints::USERS_LIST,
        &format!("/users?limit={limit}"),
        json,
    )
    .await?
    else {
        return Ok(());
    };

    if users.is_empty() {
        println!("no users");
        return Ok(());
    }

    for (i, u) in users.iter().enumerate() {
        let name = u.name.as_deref().unwrap_or("(no name)");
        let status = if u.deactivated_at.is_some() {
            " [deactivated]"
        } else {
            ""
        };
        println!("{} ({}){}", u.email, u.role, status);
        println!("  id:   {}", u.id);
        println!("  name: {name}");
        if i < users.len() - 1 {
            println!();
        }
    }

    if users.len() > 1 {
        println!("\n{} users", users.len());
    }

    Ok(())
}

pub async fn set_role(
    server: &str,
    token: &str,
    id: &str,
    role: &str,
    json: bool,
) -> anyhow::Result<()> {
    let role: Role = parse_role(role)?;
    let body = SetUserRoleRequest { role };
    let Some(user) = api::send_at(
        server,
        token,
        &endpoints::USERS_SET_ROLE,
        &format!("/users/{id}/role"),
        &body,
        json,
    )
    .await?
    else {
        return Ok(());
    };
    println!("updated {} to role '{}'", user.email, user.role);
    Ok(())
}

pub async fn deactivate(server: &str, token: &str, id: &str, json: bool) -> anyhow::Result<()> {
    let Some(user) = api::call_at(
        server,
        token,
        &endpoints::USERS_DEACTIVATE,
        &format!("/users/{id}/deactivate"),
        json,
    )
    .await?
    else {
        return Ok(());
    };
    println!("deactivated {}", user.email);
    Ok(())
}

pub async fn reactivate(server: &str, token: &str, id: &str, json: bool) -> anyhow::Result<()> {
    let Some(user) = api::call_at(
        server,
        token,
        &endpoints::USERS_REACTIVATE,
        &format!("/users/{id}/reactivate"),
        json,
    )
    .await?
    else {
        return Ok(());
    };
    println!("reactivated {}", user.email);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_role_accepts_admin() {
        assert_eq!(parse_role("admin").unwrap(), Role::Admin);
    }

    #[test]
    fn parse_role_accepts_member() {
        assert_eq!(parse_role("member").unwrap(), Role::Member);
    }

    #[test]
    fn parse_role_rejects_invalid() {
        assert!(parse_role("moderator").is_err());
        assert!(parse_role("superadmin").is_err());
        assert!(parse_role("user").is_err());
    }

    #[test]
    fn parse_role_rejects_empty() {
        assert!(parse_role("").is_err());
    }

    #[test]
    fn parse_role_rejects_wrong_case() {
        assert!(parse_role("Admin").is_err());
        assert!(parse_role("ADMIN").is_err());
        assert!(parse_role("Member").is_err());
    }

    #[test]
    fn parse_role_error_message_includes_input() {
        let err = parse_role("bogus").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("bogus"), "error should include input: {msg}");
        assert!(msg.contains("admin"), "error should hint valid values: {msg}");
    }
}
