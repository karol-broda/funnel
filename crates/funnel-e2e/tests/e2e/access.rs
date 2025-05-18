use reqwest::Method;

use crate::harness::TestEnv;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test(flavor = "multi_thread")]
async fn basic_auth_rejects_without_credentials() -> TestResult {
    let env = TestEnv::start_with_auth("admin", "secret").await?;

    let resp = env.tunnel_request(Method::GET, "/hello").send().await?;

    assert_eq!(resp.status(), 401);
    assert!(resp.headers().contains_key("www-authenticate"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn basic_auth_rejects_wrong_credentials() -> TestResult {
    let env = TestEnv::start_with_auth("admin", "secret").await?;

    let resp = env
        .tunnel_request(Method::GET, "/hello")
        .basic_auth("admin", Some("wrong"))
        .send()
        .await?;

    assert_eq!(resp.status(), 401);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn basic_auth_accepts_valid_credentials() -> TestResult {
    let env = TestEnv::start_with_auth("admin", "secret").await?;

    let resp = env
        .tunnel_request(Method::GET, "/hello")
        .basic_auth("admin", Some("secret"))
        .send()
        .await?;

    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await?, "hello from local service");
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
