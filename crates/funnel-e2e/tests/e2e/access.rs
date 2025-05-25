use futures_util::{SinkExt, StreamExt};
use reqwest::Method;
use tokio_tungstenite::tungstenite;

use crate::harness::{TestEnv, proxy_basic_auth};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn websocket_request(
    env: &TestEnv,
    path: &str,
    proxy_authorization: Option<&str>,
) -> Result<tungstenite::http::Request<()>, tungstenite::http::Error> {
    let url = format!("ws://127.0.0.1:{}{path}", env.http_port);
    let mut builder = tungstenite::http::Request::builder()
        .uri(&url)
        .header("host", &env.host_header)
        .header("connection", "Upgrade")
        .header("upgrade", "websocket")
        .header("sec-websocket-version", "13")
        .header(
            "sec-websocket-key",
            tungstenite::handshake::client::generate_key(),
        );
    if let Some(proxy_authorization) = proxy_authorization {
        builder = builder.header("proxy-authorization", proxy_authorization);
    }
    builder.body(())
}

#[tokio::test(flavor = "multi_thread")]
async fn basic_auth_rejects_without_credentials() -> TestResult {
    let env = TestEnv::start_with_auth("admin", "secret").await?;

    let resp = env.tunnel_request(Method::GET, "/hello").send().await?;

    assert_eq!(resp.status(), 407);
    assert!(resp.headers().contains_key("proxy-authenticate"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn basic_auth_rejects_wrong_credentials() -> TestResult {
    let env = TestEnv::start_with_auth("admin", "secret").await?;

    let resp = env
        .tunnel_request(Method::GET, "/hello")
        .header("proxy-authorization", proxy_basic_auth("admin", "wrong"))
        .send()
        .await?;

    assert_eq!(resp.status(), 407);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn basic_auth_accepts_valid_credentials() -> TestResult {
    let env = TestEnv::start_with_auth("admin", "secret").await?;

    let resp = env
        .tunnel_request(Method::GET, "/hello")
        .header("proxy-authorization", proxy_basic_auth("admin", "secret"))
        .send()
        .await?;

    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await?, "hello from local service");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn proxy_auth_is_stripped_and_application_authorization_passes_through() -> TestResult {
    let env = TestEnv::start_with_auth("admin", "secret").await?;

    let resp = env
        .tunnel_request(Method::GET, "/headers")
        .header("proxy-authorization", proxy_basic_auth("admin", "secret"))
        .header("authorization", "Bearer app-token")
        .send()
        .await?;

    assert_eq!(resp.status(), 200);
    let received: serde_json::Value = resp.json().await?;
    assert_eq!(received["authorization"], "Bearer app-token");
    assert!(received.get("proxy-authorization").is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn basic_scheme_rejects_without_credentials_with_401() -> TestResult {
    let env =
        TestEnv::start_with_client_args(&["--auth", "admin:secret", "--auth-scheme", "basic"])
            .await?;

    let resp = env.tunnel_request(Method::GET, "/hello").send().await?;

    assert_eq!(resp.status(), 401);
    assert!(resp.headers().contains_key("www-authenticate"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn basic_scheme_accepts_authorization_and_strips_it() -> TestResult {
    let env =
        TestEnv::start_with_client_args(&["--auth", "admin:secret", "--auth-scheme", "basic"])
            .await?;

    let resp = env
        .tunnel_request(Method::GET, "/headers")
        .basic_auth("admin", Some("secret"))
        .send()
        .await?;

    assert_eq!(resp.status(), 200);
    let received: serde_json::Value = resp.json().await?;
    assert!(received.get("authorization").is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn ip_allowlist_permits_listed_peer() -> TestResult {
    let env = TestEnv::start_with_client_args(&["--allow-ip", "127.0.0.1/32"]).await?;

    let resp = env.tunnel_request(Method::GET, "/hello").send().await?;

    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await?, "hello from local service");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn ip_allowlist_blocks_unlisted_peer() -> TestResult {
    let env = TestEnv::start_with_client_args(&["--allow-ip", "10.0.0.0/8"]).await?;

    let resp = env.tunnel_request(Method::GET, "/hello").send().await?;

    assert_eq!(resp.status(), 403);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn basic_auth_rejects_websocket_upgrade_without_credentials() -> TestResult {
    let env = TestEnv::start_with_auth("admin", "secret").await?;

    let request = websocket_request(&env, "/ws-echo", None)?;
    let result = tokio_tungstenite::connect_async(request).await;

    match result {
        Err(tungstenite::Error::Http(response)) => {
            assert_eq!(response.status(), 407);
            Ok(())
        }
        Err(other) => Err(format!("expected http 407, got error: {other}").into()),
        Ok(_) => Err("expected upgrade to be rejected, but it succeeded".into()),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn basic_auth_allows_websocket_upgrade_with_credentials() -> TestResult {
    let env = TestEnv::start_with_auth("admin", "secret").await?;

    let proxy_authorization = proxy_basic_auth("admin", "secret");
    let request = websocket_request(&env, "/ws-echo", Some(&proxy_authorization))?;
    let (mut socket, _response) = tokio_tungstenite::connect_async(request).await?;

    socket
        .send(tungstenite::Message::Text("ping".into()))
        .await?;
    let echoed = socket.next().await.ok_or("no message received")??;
    assert_eq!(echoed.into_text()?, "ping");

    socket.close(None).await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn expiry_closes_tunnel_after_duration() -> TestResult {
    let expiry_seconds = 3;
    let env =
        TestEnv::start_with_client_args(&["--expires", &format!("{expiry_seconds}s")]).await?;

    let before = env.tunnel_request(Method::GET, "/hello").send().await?;
    assert_eq!(before.status(), 200);

    tokio::time::sleep(std::time::Duration::from_secs(expiry_seconds + 2)).await;

    let after = env.tunnel_request(Method::GET, "/hello").send().await?;
    assert!(
        after.status() == reqwest::StatusCode::NOT_FOUND
            || after.status() == reqwest::StatusCode::GONE,
        "expected tunnel to be gone or expired, got {}",
        after.status()
    );
    Ok(())
}
