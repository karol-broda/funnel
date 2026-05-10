use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use crate::harness::{binary_path, free_port, log_file, wait_for_tcp};

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct AuthTestEnv {
    server_process: Child,
    server_log: PathBuf,
    http_port: u16,
    seed_key: String,
    client: reqwest::Client,
}

impl AuthTestEnv {
    async fn start() -> Result<Self, Box<dyn std::error::Error>> {
        let http_port = free_port()?;
        let quic_port = free_port()?;

        let (stderr_file, log_path) = log_file("auth-server")?;

        let mut child = Command::new(binary_path("funnel-server")?)
            .args([
                "--port",
                &http_port.to_string(),
                "--quic-port",
                &quic_port.to_string(),
                "--host",
                "127.0.0.1",
                "--seed-api-key",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::from(stderr_file))
            .spawn()?;

        let stdout = child.stdout.take().ok_or("stdout not piped")?;
        let mut reader = BufReader::new(stdout);
        let mut seed_key = String::new();
        reader.read_line(&mut seed_key)?;
        let seed_key = seed_key.trim().to_string();

        wait_for_tcp(http_port).await;

        Ok(Self {
            server_process: child,
            server_log: log_path,
            http_port,
            seed_key,
            client: reqwest::Client::new(),
        })
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", self.http_port, path)
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.seed_key)
    }

    fn dump_logs(&self) {
        if let Ok(content) = std::fs::read_to_string(&self.server_log)
            && !content.is_empty()
        {
            eprintln!("\n--- auth server stderr ---\n{content}\n--- end ---\n");
        }
    }
}

impl Drop for AuthTestEnv {
    fn drop(&mut self) {
        let _ = self.server_process.kill();
        let _ = self.server_process.wait();

        if std::thread::panicking() {
            self.dump_logs();
        }

        let _ = std::fs::remove_file(&self.server_log);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn keys_require_auth() -> TestResult {
    let env = AuthTestEnv::start().await?;

    let get = env.client.get(env.url("/api/keys")).send().await?;
    assert_eq!(get.status(), 401);

    let post = env
        .client
        .post(env.url("/api/keys"))
        .json(&serde_json::json!({"name": "test"}))
        .send()
        .await?;
    assert_eq!(post.status(), 401);

    let delete = env
        .client
        .delete(env.url("/api/keys/00000000-0000-0000-0000-000000000000"))
        .send()
        .await?;
    assert_eq!(delete.status(), 401);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_bearer_token_rejected() -> TestResult {
    let env = AuthTestEnv::start().await?;

    let resp = env
        .client
        .get(env.url("/api/keys"))
        .header("authorization", "Bearer fnl_bogustoken1234567890")
        .send()
        .await?;
    assert_eq!(resp.status(), 401);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn create_and_list_keys() -> TestResult {
    let env = AuthTestEnv::start().await?;

    let resp = env
        .client
        .post(env.url("/api/keys"))
        .header("authorization", env.auth_header())
        .json(&serde_json::json!({"name": "my-key"}))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await?;
    assert!(body["key"].as_str().is_some());
    assert_eq!(body["info"]["name"], "my-key");

    // seed key + new key
    let resp = env
        .client
        .get(env.url("/api/keys"))
        .header("authorization", env.auth_header())
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let keys: Vec<serde_json::Value> = resp.json().await?;
    assert_eq!(keys.len(), 2);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn revoke_key() -> TestResult {
    let env = AuthTestEnv::start().await?;

    let resp = env
        .client
        .post(env.url("/api/keys"))
        .header("authorization", env.auth_header())
        .json(&serde_json::json!({"name": "to-revoke"}))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await?;
    let key_id = body["info"]["id"].as_str().ok_or("missing key id")?;

    let resp = env
        .client
        .delete(env.url(&format!("/api/keys/{key_id}")))
        .header("authorization", env.auth_header())
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["revoked"], true);

    // only seed key remains
    let resp = env
        .client
        .get(env.url("/api/keys"))
        .header("authorization", env.auth_header())
        .send()
        .await?;
    let keys: Vec<serde_json::Value> = resp.json().await?;
    assert_eq!(keys.len(), 1);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn revoked_key_cannot_authenticate() -> TestResult {
    let env = AuthTestEnv::start().await?;

    // create a second key
    let resp = env
        .client
        .post(env.url("/api/keys"))
        .header("authorization", env.auth_header())
        .json(&serde_json::json!({"name": "ephemeral"}))
        .send()
        .await?;
    let body: serde_json::Value = resp.json().await?;
    let new_key = body["key"].as_str().ok_or("missing key")?.to_string();
    let new_key_id = body["info"]["id"].as_str().ok_or("missing key id")?;

    // verify it works
    let resp = env
        .client
        .get(env.url("/api/keys"))
        .header("authorization", format!("Bearer {new_key}"))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);

    // revoke it
    env.client
        .delete(env.url(&format!("/api/keys/{new_key_id}")))
        .header("authorization", env.auth_header())
        .send()
        .await?;

    // revoked key should be rejected
    let resp = env
        .client
        .get(env.url("/api/keys"))
        .header("authorization", format!("Bearer {new_key}"))
        .send()
        .await?;
    assert_eq!(resp.status(), 401);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn me_requires_auth() -> TestResult {
    let env = AuthTestEnv::start().await?;

    let resp = env.client.get(env.url("/api/me")).send().await?;
    assert_eq!(resp.status(), 401);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn me_returns_seed_user() -> TestResult {
    let env = AuthTestEnv::start().await?;

    // seed key creates a proper system user
    let resp = env
        .client
        .get(env.url("/api/me"))
        .header("authorization", env.auth_header())
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["email"], "system@funnel.local");
    assert_eq!(body["role"], "admin");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn oauth_authorize_returns_404_when_not_configured() -> TestResult {
    let env = AuthTestEnv::start().await?;

    let resp = env
        .client
        .get(env.url("/auth/github/authorize?cli_port=9999"))
        .send()
        .await?;
    assert_eq!(resp.status(), 404);

    let body = resp.text().await?;
    assert!(body.contains("oauth not configured"));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn oauth_callback_returns_404_when_not_configured() -> TestResult {
    let env = AuthTestEnv::start().await?;

    let resp = env
        .client
        .get(env.url("/auth/github/callback?code=test&state=test"))
        .send()
        .await?;
    assert_eq!(resp.status(), 404);

    Ok(())
}
