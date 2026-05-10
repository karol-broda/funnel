use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::extract::Query;
use axum::response::Html;
use axum::routing::get;
use serde::Deserialize;
use tokio::sync::Notify;

use crate::config;

#[derive(Deserialize)]
struct CallbackParams {
    token: String,
}

pub async fn login(context_name: &str, provider: &str) -> anyhow::Result<()> {
    let cfg = config::load()?;
    let resolved = config::resolve(&cfg, Some(context_name))?;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let local_addr = listener.local_addr()?;
    let port = local_addr.port();

    let login_url = format!(
        "{}/auth/{provider}/authorize?cli_port={port}",
        resolved.server
    );

    println!("opening browser to log in...");
    if !open_browser(&login_url) {
        println!("could not open browser, visit this URL manually:");
        println!("  {login_url}");
    }

    let shutdown = Arc::new(Notify::new());
    let shutdown_signal = Arc::clone(&shutdown);
    let context = context_name.to_string();

    let app = Router::new().route(
        "/callback",
        get(move |Query(params): Query<CallbackParams>| {
            let shutdown = shutdown_signal.clone();
            let context = context.clone();
            async move {
                let result = config::set_token(&context, &params.token);
                shutdown.notify_one();
                match result {
                    Ok(()) => Html(
                                            r#"<html>
                                                <body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; max-width: 600px; margin: 40px auto; padding: 0 20px; color: #111827;">
                                                    <h1 style="font-size: 1.5rem; font-weight: 600;">Login successful</h1>
                                                    <p style="color: #4b5563;">You can close this tab and return to the app.</p>
                                                </body>
                                            </html>"#.to_string(),
                                        ),
                                        Err(e) => Html(format!(
                                            r#"<html>
                                                <body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; max-width: 600px; margin: 40px auto; padding: 0 20px; color: #111827;">
                                                    <h1 style="font-size: 1.5rem; font-weight: 600; color: #b91c1c;">Error</h1>
                                                    <p style="color: #4b5563;">Failed to save token: {e}</p>
                                                </body>
                                            </html>"#
                                        )),
                }
            }
        }),
    );

    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    );

    let shutdown_wait = shutdown.clone();
    tokio::select! {
        res = server => { res?; }
        () = shutdown_wait.notified() => {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    };

    println!("logged in successfully, token saved to context '{context_name}'");
    Ok(())
}

fn open_browser(url: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(url).spawn().is_ok()
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .is_ok()
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", url])
            .spawn()
            .is_ok()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = url;
        false
    }
}
