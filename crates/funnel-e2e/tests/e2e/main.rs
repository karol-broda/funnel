mod auth;
mod harness;

use harness::TestEnv;
use reqwest::Method;

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
