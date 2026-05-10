mod auth;
mod harness;

use futures_util::{SinkExt, StreamExt};
use harness::TestEnv;
use reqwest::Method;
use tokio_tungstenite::tungstenite;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test(flavor = "multi_thread")]
async fn basic_get() -> TestResult {
    let env = TestEnv::start().await?;

    let resp = env.tunnel_request(Method::GET, "/hello").send().await?;

    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await?, "hello from local service");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn post_echo() -> TestResult {
    let env = TestEnv::start().await?;

    let resp = env
        .tunnel_request(Method::POST, "/echo")
        .body("test body")
        .send()
        .await?;

    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await?, "echo: test body");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn large_body() -> TestResult {
    let env = TestEnv::start().await?;
    let size = 2_500_000;
    let payload = vec![b'x'; size];

    let resp = env
        .tunnel_request(Method::POST, "/large")
        .body(payload)
        .send()
        .await?;

    assert_eq!(resp.status(), 200);
    let body = resp.bytes().await?;
    assert_eq!(body.len(), size);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_requests() -> TestResult {
    let env = TestEnv::start().await?;
    let mut set = tokio::task::JoinSet::new();

    for _ in 0..10 {
        let client = env.client.clone();
        let url = format!("http://127.0.0.1:{}/hello", env.http_port);
        let host = env.host_header.clone();

        set.spawn(async move { client.get(&url).header("host", &host).send().await });
    }

    let mut ok_count = 0;
    while let Some(result) = set.join_next().await {
        let resp = result??;
        assert_eq!(resp.status(), 200);
        ok_count += 1;
    }
    assert_eq!(ok_count, 10);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn response_headers() -> TestResult {
    let env = TestEnv::start().await?;

    let resp = env.tunnel_request(Method::GET, "/headers").send().await?;

    assert_eq!(resp.status(), 200);
    let custom = resp.headers().get("x-custom-header");
    assert_eq!(custom.and_then(|v| v.to_str().ok()), Some("test-value"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn metrics_endpoint() -> TestResult {
    let env = TestEnv::start().await?;

    let resp = env
        .client
        .get(env.server_url("/api/metrics"))
        .send()
        .await?;

    assert_eq!(resp.status(), 200);
    let body = resp.text().await?;
    assert!(body.contains("funnel_requests_total"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn unknown_tunnel_returns_404() -> TestResult {
    let env = TestEnv::start().await?;
    let url = format!("http://127.0.0.1:{}/hello", env.http_port);

    let resp = env
        .client
        .get(&url)
        .header("host", format!("nonexistent.localhost:{}", env.http_port))
        .send()
        .await?;

    assert_eq!(resp.status(), 404);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn websocket_echo() -> TestResult {
    let env = TestEnv::start().await?;

    let request = ws_request(&env, "/ws-echo")?;
    let (mut ws, _resp) = tokio_tungstenite::connect_async(request).await?;

    ws.send(tungstenite::Message::Text("hello websocket".into())).await?;

    let msg = ws.next().await
        .ok_or("no message received")??;

    assert_eq!(msg.into_text()?, "hello websocket");

    ws.close(None).await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn websocket_multiple_messages() -> TestResult {
    let env = TestEnv::start().await?;

    let request = ws_request(&env, "/ws-echo")?;
    let (mut ws, _) = tokio_tungstenite::connect_async(request).await?;

    for i in 0..5 {
        let msg = format!("msg {i}");
        ws.send(tungstenite::Message::Text(msg.clone().into())).await?;

        let resp = ws.next().await
            .ok_or("no message received")??;
        assert_eq!(resp.into_text()?, msg);
    }

    ws.close(None).await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn websocket_binary_data() -> TestResult {
    let env = TestEnv::start().await?;

    let request = ws_request(&env, "/ws-echo")?;
    let (mut ws, _) = tokio_tungstenite::connect_async(request).await?;

    let payload = vec![0u8, 1, 2, 255, 254, 253];
    ws.send(tungstenite::Message::Binary(payload.clone().into())).await?;

    let resp = ws.next().await
        .ok_or("no message received")??;
    assert_eq!(resp.into_data().to_vec(), payload);

    ws.close(None).await?;
    Ok(())
}

fn ws_request(
    env: &TestEnv,
    path: &str,
) -> Result<tungstenite::http::Request<()>, tungstenite::http::Error> {
    let url = format!("ws://127.0.0.1:{}{path}", env.http_port);
    tungstenite::http::Request::builder()
        .uri(&url)
        .header("host", &env.host_header)
        .header("connection", "Upgrade")
        .header("upgrade", "websocket")
        .header("sec-websocket-version", "13")
        .header(
            "sec-websocket-key",
            tungstenite::handshake::client::generate_key(),
        )
        .body(())
}
