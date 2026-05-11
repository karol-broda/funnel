use funnel_core::protocol::PROTOCOL_VERSION;
use serde::{Deserialize, Serialize};

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

pub async fn run(server: &str, token: &str, command: Command) -> anyhow::Result<()> {
    match command {
        Command::List { limit } => list(server, token, limit).await,
        Command::SetRole { id, role } => set_role(server, token, &id, &role).await,
        Command::Deactivate { id } => deactivate(server, token, &id).await,
        Command::Reactivate { id } => reactivate(server, token, &id).await,
    }
}

#[derive(Deserialize)]
struct User {
    id: String,
    email: String,
    name: Option<String>,
    role: String,
    deactivated_at: Option<String>,
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

pub async fn list(server: &str, token: &str, limit: u32) -> anyhow::Result<()> {
    let resp = client(token)
        .get(format!("{server}/api/v{PROTOCOL_VERSION}/users?limit={limit}"))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("server returned {status}: {body}");
    }

    let users: Vec<User> = resp.json().await?;

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
        println!("{} ({}){}",  u.email, u.role, status);
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

#[derive(Serialize)]
struct SetRoleRequest {
    role: String,
}

pub async fn set_role(server: &str, token: &str, id: &str, role: &str) -> anyhow::Result<()> {
    let resp = client(token)
        .put(format!("{server}/api/v{PROTOCOL_VERSION}/users/{id}/role"))
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

    let user: User = resp.json().await?;
    println!("updated {} to role '{}'", user.email, user.role);
    Ok(())
}

pub async fn deactivate(server: &str, token: &str, id: &str) -> anyhow::Result<()> {
    let resp = client(token)
        .post(format!("{server}/api/v{PROTOCOL_VERSION}/users/{id}/deactivate"))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("server returned {status}: {body}");
    }

    let user: User = resp.json().await?;
    println!("deactivated {}", user.email);
    Ok(())
}

pub async fn reactivate(server: &str, token: &str, id: &str) -> anyhow::Result<()> {
    let resp = client(token)
        .post(format!("{server}/api/v{PROTOCOL_VERSION}/users/{id}/reactivate"))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("server returned {status}: {body}");
    }

    let user: User = resp.json().await?;
    println!("reactivated {}", user.email);
    Ok(())
}
