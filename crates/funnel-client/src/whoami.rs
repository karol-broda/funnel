use serde::Deserialize;

#[derive(Deserialize)]
struct UserInfo {
    email: String,
    name: Option<String>,
    role: String,
}

pub async fn run(server: &str, token: &str, context_name: &str) -> anyhow::Result<()> {
    let resp = reqwest::Client::new()
        .get(format!("{server}/api/me"))
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await?;

    if resp.status().as_u16() == 404 {
        println!("no user profile found (api key without linked user)");
        println!("  context: {context_name} ({server})");
        return Ok(());
    }

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("server returned {status}: {body}");
    }

    let user: UserInfo = resp.json().await?;

    if let Some(name) = &user.name {
        println!("{name} <{}>", user.email);
    } else {
        println!("{}", user.email);
    }
    println!("  role:    {}", user.role);
    println!("  context: {context_name} ({server})");

    Ok(())
}
