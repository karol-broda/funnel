use std::fs::File;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use tokio::task::JoinHandle;

const TUNNEL_ID: &str = "e2etest";
const READY_TIMEOUT: Duration = Duration::from_secs(30);
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const MOCK_BODY_LIMIT: usize = 4 * 1024 * 1024;

pub struct TestEnv {
    server_process: Child,
    client_process: Child,
    local_server_handle: JoinHandle<()>,
    server_log: PathBuf,
    client_log: PathBuf,
    pub http_port: u16,
    pub host_header: String,
    pub client: reqwest::Client,
}

impl TestEnv {
    pub async fn start() -> Result<Self, Box<dyn std::error::Error>> {
        let (local_handle, local_port) = start_mock_server()?;
        let http_port = free_port()?;
        let quic_port = free_port()?;
        let host_header = format!("{TUNNEL_ID}.localhost:{http_port}");

        let (server_process, server_log) = start_server_process(http_port, quic_port)?;
        wait_for_tcp(http_port).await;

        let (client_process, client_log) =
            start_client_process(local_port, http_port, quic_port, TUNNEL_ID)?;

        let client = reqwest::Client::new();
        wait_for_tunnel(&client, http_port, &host_header).await;

        Ok(Self {
            server_process,
            client_process,
            local_server_handle: local_handle,
            server_log,
            client_log,
            http_port,
            host_header,
            client,
        })
    }

    pub fn tunnel_request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("http://127.0.0.1:{}{}", self.http_port, path);
        self.client
            .request(method, &url)
            .header("host", &self.host_header)
    }

    pub fn server_url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", self.http_port, path)
    }

    fn dump_logs(&self) {
        if let Ok(content) = std::fs::read_to_string(&self.server_log)
            && !content.is_empty()
        {
            eprintln!("\n--- funnel-server stderr ---");
            eprintln!("{content}");
            eprintln!("--- end funnel-server stderr ---\n");
        }
        if let Ok(content) = std::fs::read_to_string(&self.client_log)
            && !content.is_empty()
        {
            eprintln!("\n--- funnel-client stderr ---");
            eprintln!("{content}");
            eprintln!("--- end funnel-client stderr ---\n");
        }
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = self.server_process.kill();
        let _ = self.client_process.kill();
        self.local_server_handle.abort();
        let _ = self.server_process.wait();
        let _ = self.client_process.wait();

        if std::thread::panicking() {
            self.dump_logs();
        }

        let _ = std::fs::remove_file(&self.server_log);
        let _ = std::fs::remove_file(&self.client_log);
    }
}

pub fn free_port() -> Result<u16, std::io::Error> {
    Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
}

pub fn binary_path(name: &str) -> Result<PathBuf, String> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .ok_or("could not resolve workspace root from CARGO_MANIFEST_DIR")?;
    let path = workspace_root.join("target").join("debug").join(name);
    if !path.exists() {
        return Err(format!(
            "binary not found at {}: run `cargo build -p funnel-server -p funnel-client` first",
            path.display()
        ));
    }
    Ok(path)
}

pub fn log_file(name: &str) -> Result<(File, PathBuf), std::io::Error> {
    let path = std::env::temp_dir().join(format!("funnel-{name}-{}.log", std::process::id()));
    let file = File::create(&path)?;
    Ok((file, path))
}

fn start_mock_server() -> Result<(JoinHandle<()>, u16), std::io::Error> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    listener.set_nonblocking(true)?;

    let tokio_listener = tokio::net::TcpListener::from_std(listener)?;

    let handle = tokio::spawn(async move {
        let app = mock_app();
        if let Err(e) = axum::serve(tokio_listener, app).await {
            eprintln!("mock server error: {e}");
        }
    });

    Ok((handle, port))
}

fn mock_app() -> Router {
    Router::new()
        .route("/hello", get(|| async { "hello from local service" }))
        .route(
            "/echo",
            post(|body: String| async move { format!("echo: {body}") }),
        )
        .route(
            "/headers",
            get(|headers: axum::http::HeaderMap| async move {
                let map: std::collections::HashMap<String, String> = headers
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                    .collect();
                ([("x-custom-header", "test-value")], axum::Json(map))
            }),
        )
        .route(
            "/large",
            post(|body: axum::body::Bytes| async move { body }),
        )
        .layer(DefaultBodyLimit::max(MOCK_BODY_LIMIT))
}

fn start_server_process(
    http_port: u16,
    quic_port: u16,
) -> Result<(Child, PathBuf), Box<dyn std::error::Error>> {
    let (stderr_file, log_path) = log_file("server")?;

    let child = Command::new(binary_path("funnel-server")?)
        .args([
            "--port",
            &http_port.to_string(),
            "--quic-port",
            &quic_port.to_string(),
            "--host",
            "127.0.0.1",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::from(stderr_file))
        .spawn()?;

    Ok((child, log_path))
}

fn start_client_process(
    local_port: u16,
    server_http_port: u16,
    quic_port: u16,
    tunnel_id: &str,
) -> Result<(Child, PathBuf), Box<dyn std::error::Error>> {
    let (stderr_file, log_path) = log_file("client")?;

    let child = Command::new(binary_path("funnel-client")?)
        .args([
            "http",
            &format!("127.0.0.1:{local_port}"),
            "--server",
            &format!("http://127.0.0.1:{server_http_port}"),
            "--id",
            tunnel_id,
            "--quic-port",
            &quic_port.to_string(),
            "--insecure",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::from(stderr_file))
        .spawn()?;

    Ok((child, log_path))
}

pub async fn wait_for_tcp(port: u16) {
    let deadline = tokio::time::Instant::now() + READY_TIMEOUT;
    loop {
        assert!(
            tokio::time::Instant::now() <= deadline,
            "tcp port {port} did not become ready"
        );
        if tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .is_ok()
        {
            return;
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

async fn wait_for_tunnel(client: &reqwest::Client, http_port: u16, host_header: &str) {
    let url = format!("http://127.0.0.1:{http_port}/hello");
    let deadline = tokio::time::Instant::now() + READY_TIMEOUT;
    loop {
        assert!(
            tokio::time::Instant::now() <= deadline,
            "tunnel did not become ready"
        );
        match client.get(&url).header("host", host_header).send().await {
            Ok(resp) if resp.status().is_success() => return,
            _ => tokio::time::sleep(POLL_INTERVAL).await,
        }
    }
}
