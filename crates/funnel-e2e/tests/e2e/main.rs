mod harness;

use harness::TestEnv;
use reqwest::Method;

#[tokio::test(flavor = "multi_thread")]
async fn e2e() {
    let env = TestEnv::start().await;

    basic_get(&env).await;
    post_echo(&env).await;
    large_body(&env).await;
    concurrent_requests(&env).await;
    response_headers(&env).await;
    metrics_endpoint(&env).await;
    unknown_tunnel_404(&env).await;
}

async fn basic_get(env: &TestEnv) {
    let resp = env
        .tunnel_request(Method::GET, "/hello")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200, "basic_get: expected 200");
    assert_eq!(
        resp.text().await.unwrap(),
        "hello from local service",
        "basic_get: unexpected body"
    );
}

async fn post_echo(env: &TestEnv) {
    let resp = env
        .tunnel_request(Method::POST, "/echo")
        .body("test body")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200, "post_echo: expected 200");
    assert_eq!(
        resp.text().await.unwrap(),
        "echo: test body",
        "post_echo: unexpected body"
    );
}

async fn large_body(env: &TestEnv) {
    let size = 2_500_000;
    let payload = vec![b'x'; size];
    let resp = env
        .tunnel_request(Method::POST, "/large")
        .body(payload)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200, "large_body: expected 200");
    let body = resp.bytes().await.unwrap();
    assert_eq!(body.len(), size, "large_body: response size mismatch");
}

async fn concurrent_requests(env: &TestEnv) {
    let mut set = tokio::task::JoinSet::new();

    for _ in 0..10 {
        let client = env.client.clone();
        let url = format!("http://127.0.0.1:{}/hello", env.http_port);
        let host = env.host_header.clone();

        set.spawn(async move {
            client
                .get(&url)
                .header("host", &host)
                .send()
                .await
                .unwrap()
        });
    }

    let mut ok_count = 0;
    while let Some(result) = set.join_next().await {
        let resp = result.unwrap();
        assert_eq!(resp.status(), 200, "concurrent_requests: expected 200");
        ok_count += 1;
    }
    assert_eq!(ok_count, 10, "concurrent_requests: expected 10 responses");
}

async fn response_headers(env: &TestEnv) {
    let resp = env
        .tunnel_request(Method::GET, "/headers")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200, "response_headers: expected 200");

    let custom = resp.headers().get("x-custom-header");
    assert_eq!(
        custom.and_then(|v| v.to_str().ok()),
        Some("test-value"),
        "response_headers: missing or wrong x-custom-header"
    );
}

async fn metrics_endpoint(env: &TestEnv) {
    let resp = env
        .client
        .get(&env.server_url("/api/metrics"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200, "metrics_endpoint: expected 200");
    let body = resp.text().await.unwrap();
    assert!(
        body.contains("funnel_requests_total"),
        "metrics_endpoint: expected funnel_requests_total in prometheus output"
    );
}

async fn unknown_tunnel_404(env: &TestEnv) {
    let url = format!("http://127.0.0.1:{}/hello", env.http_port);
    let resp = env
        .client
        .get(&url)
        .header("host", format!("nonexistent.localhost:{}", env.http_port))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 404, "unknown_tunnel_404: expected 404");
}
