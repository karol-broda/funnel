use funnel_core::api as endpoints;

use super::api;

pub async fn run(server: &str, token: &str, context_name: &str, json: bool) -> anyhow::Result<()> {
    let Some(user) = api::call(server, token, &endpoints::ME, json).await? else {
        return Ok(());
    };

    if let Some(name) = &user.name {
        println!("{name} <{}>", user.email);
    } else {
        println!("{}", user.email);
    }
    println!("  role:    {}", user.role);
    println!("  context: {context_name} ({server})");

    Ok(())
}
