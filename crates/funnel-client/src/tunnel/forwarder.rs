use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

use funnel_core::protocol::request::{HttpRequest, HttpResponse};

const HOP_BY_HOP_HEADERS: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
    "proxy-connection",
];

const MAX_IDLE_CONNECTIONS: usize = 8;

pub struct PooledConnection {
    sender: hyper::client::conn::http1::SendRequest<Full<Bytes>>,
}

pub struct Forwarder {
    local_addr: String,
    pool: Mutex<Vec<PooledConnection>>,
}

pub struct UpgradeResult {
    pub meta: HttpResponse,
    pub upgraded: hyper::upgrade::Upgraded,
}

pub enum ForwardUpgradeResult {
    Upgraded(UpgradeResult),
    Rejected(HttpResponse, Vec<u8>),
}

pub enum ForwardResult {
    Success {
        meta: HttpResponse,
        body: Incoming,
        conn: PooledConnection,
    },
    LocalError {
        meta: HttpResponse,
        body: Vec<u8>,
    },
}

impl Forwarder {
    pub const fn new(local_addr: String) -> Self {
        Self {
            local_addr,
            pool: Mutex::new(Vec::new()),
        }
    }

    pub fn local_addr(&self) -> &str {
        &self.local_addr
    }

    async fn acquire(&self) -> anyhow::Result<PooledConnection> {
        loop {
            let candidate = {
                let mut pool = self.pool.lock().unwrap_or_else(PoisonError::into_inner);
                pool.pop()
            };
            match candidate {
                Some(conn) if conn.sender.is_ready() => return Ok(conn),
                Some(_) => {} // stale, discard and try next
                None => break,
            }
        }

        let stream = TcpStream::connect(&self.local_addr).await?;
        let io = TokioIo::new(stream);
        let (sender, conn) = hyper::client::conn::http1::handshake(io).await?;
        tokio::spawn(async move {
            if let Err(e) = conn.await {
                tracing::debug!(error = %e, "connection task ended");
            }
        });

        Ok(PooledConnection { sender })
    }

    pub fn release(&self, conn: PooledConnection) {
        if !conn.sender.is_ready() {
            return;
        }
        let mut pool = self.pool.lock().unwrap_or_else(PoisonError::into_inner);
        if pool.len() < MAX_IDLE_CONNECTIONS {
            pool.push(conn);
        }
    }

    pub async fn forward(&self, meta: &HttpRequest, body: Bytes) -> ForwardResult {
        match self.try_forward(meta, body).await {
            Ok((resp_meta, incoming, conn)) => ForwardResult::Success {
                meta: resp_meta,
                body: incoming,
                conn,
            },
            Err(e) => {
                let (resp_meta, resp_body) =
                    error_response(502, &format!("local service error: {e}"));
                ForwardResult::LocalError {
                    meta: resp_meta,
                    body: resp_body,
                }
            }
        }
    }

    async fn try_forward(
        &self,
        meta: &HttpRequest,
        body: Bytes,
    ) -> anyhow::Result<(HttpResponse, Incoming, PooledConnection)> {
        let mut conn = self.acquire().await?;

        let req = build_request(meta, &self.local_addr, body, false)?;
        let resp = conn.sender.send_request(req).await?;

        let status = resp.status().as_u16();
        let headers = collect_response_headers(resp.headers(), false);
        let resp_meta = HttpResponse { status, headers };

        Ok((resp_meta, resp.into_body(), conn))
    }

    pub async fn forward_upgrade(
        &self,
        meta: &HttpRequest,
    ) -> anyhow::Result<ForwardUpgradeResult> {
        // upgrades are one shot, no pool reuse
        let stream = TcpStream::connect(&self.local_addr).await?;
        let io = TokioIo::new(stream);

        let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
        tokio::spawn(async move {
            if let Err(e) = conn.with_upgrades().await {
                tracing::debug!(error = %e, "upgrade connection task ended");
            }
        });

        let req = build_request(meta, &self.local_addr, Bytes::new(), true)?;
        let resp = sender.send_request(req).await?;

        let status = resp.status().as_u16();
        let is_101 = resp.status() == hyper::StatusCode::SWITCHING_PROTOCOLS;
        let headers = collect_response_headers(resp.headers(), is_101);
        let resp_meta = HttpResponse { status, headers };

        if is_101 {
            let upgraded = hyper::upgrade::on(resp).await?;
            Ok(ForwardUpgradeResult::Upgraded(UpgradeResult {
                meta: resp_meta,
                upgraded,
            }))
        } else {
            let resp_body = resp.into_body().collect().await?.to_bytes().to_vec();
            Ok(ForwardUpgradeResult::Rejected(resp_meta, resp_body))
        }
    }
}

fn build_request(
    meta: &HttpRequest,
    local_addr: &str,
    body: Bytes,
    is_upgrade: bool,
) -> anyhow::Result<hyper::Request<Full<Bytes>>> {
    let method: http::Method = meta.method.parse()?;
    let mut builder = hyper::Request::builder().method(method).uri(&meta.path);

    if let Some(headers) = builder.headers_mut() {
        for (name, values) in &meta.headers {
            if !is_upgrade && is_hop_by_hop(name) {
                continue;
            }
            if !is_upgrade && name.eq_ignore_ascii_case("accept-encoding") {
                continue;
            }
            if name.eq_ignore_ascii_case("host") {
                continue;
            }
            for value in values {
                if let (Ok(n), Ok(v)) = (
                    http::HeaderName::try_from(name.as_str()),
                    http::HeaderValue::from_str(value),
                ) {
                    headers.append(n, v);
                }
            }
        }
        headers.insert(http::header::HOST, http::HeaderValue::from_str(local_addr)?);
    }

    Ok(builder.body(Full::new(body))?)
}

fn is_hop_by_hop(name: &str) -> bool {
    HOP_BY_HOP_HEADERS
        .iter()
        .any(|h| h.eq_ignore_ascii_case(name))
}

fn collect_response_headers(
    headers: &http::HeaderMap,
    keep_hop_by_hop: bool,
) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for (name, value) in headers {
        if !keep_hop_by_hop && is_hop_by_hop(name.as_str()) {
            continue;
        }
        let val = value.to_str().unwrap_or("").to_string();
        map.entry(name.to_string()).or_default().push(val);
    }
    map
}

fn error_response(status: u16, msg: &str) -> (HttpResponse, Vec<u8>) {
    let mut headers = HashMap::new();
    headers.insert("content-type".to_string(), vec!["text/plain".to_string()]);
    (HttpResponse { status, headers }, msg.as_bytes().to_vec())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    use std::net::TcpListener;

    use axum::Router;
    use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
    use axum::response::IntoResponse;
    use axum::routing::{get, post};
    use futures_util::StreamExt;
    use http_body_util::BodyExt;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn free_port() -> u16 {
        TcpListener::bind("127.0.0.1:0")
            .and_then(|l| l.local_addr())
            .map_or(0, |a| a.port())
    }

    async fn start_test_server(app: Router) -> String {
        let port = free_port();
        let addr = format!("127.0.0.1:{port}");
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .expect("bind test server");
        tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("test server crashed");
        });
        addr
    }

    fn test_meta(method: &str, path: &str) -> HttpRequest {
        HttpRequest {
            tunnel_id: funnel_core::tunnel::id::TunnelId::new("test").unwrap(),
            remote_addr: "127.0.0.1:0".into(),
            method: method.into(),
            path: path.into(),
            headers: HashMap::new(),
            upgrade: false,
        }
    }

    fn upgrade_meta(path: &str) -> HttpRequest {
        let mut headers = HashMap::new();
        headers.insert("connection".into(), vec!["Upgrade".into()]);
        headers.insert("upgrade".into(), vec!["websocket".into()]);
        headers.insert(
            "sec-websocket-key".into(),
            vec!["dGhlIHNhbXBsZSBub25jZQ==".into()],
        );
        headers.insert("sec-websocket-version".into(), vec!["13".into()]);
        HttpRequest {
            tunnel_id: funnel_core::tunnel::id::TunnelId::new("test").unwrap(),
            remote_addr: "127.0.0.1:0".into(),
            method: "GET".into(),
            path: path.into(),
            headers,
            upgrade: true,
        }
    }

    /// helper to drain an Incoming body from a ForwardResult::Success
    async fn collect_success(result: ForwardResult) -> (HttpResponse, Vec<u8>, PooledConnection) {
        match result {
            ForwardResult::Success { meta, body, conn } => {
                let bytes = body.collect().await.unwrap().to_bytes().to_vec();
                (meta, bytes, conn)
            }
            ForwardResult::LocalError { meta, .. } => {
                panic!(
                    "expected Success, got LocalError with status {}",
                    meta.status
                );
            }
        }
    }

    #[test]
    fn hop_by_hop_detection() {
        assert!(is_hop_by_hop("Connection"));
        assert!(is_hop_by_hop("transfer-encoding"));
        assert!(is_hop_by_hop("Keep-Alive"));
        assert!(!is_hop_by_hop("Content-Type"));
        assert!(!is_hop_by_hop("Authorization"));
    }

    #[test]
    fn error_response_has_correct_status() {
        let (meta, body) = error_response(502, "test error");
        assert_eq!(meta.status, 502);
        assert_eq!(body, b"test error");
        assert!(meta.headers.contains_key("content-type"));
    }

    #[tokio::test]
    async fn forward_get_request() -> TestResult {
        let app = Router::new().route("/hello", get(|| async { "hello" }));
        let addr = start_test_server(app).await;
        let fwd = Forwarder::new(addr);

        let meta = test_meta("GET", "/hello");
        let result = fwd.forward(&meta, Bytes::new()).await;
        let (meta, body, _conn) = collect_success(result).await;

        assert_eq!(meta.status, 200);
        assert_eq!(body, b"hello");
        Ok(())
    }

    #[tokio::test]
    async fn forward_post_with_body() -> TestResult {
        let app = Router::new().route(
            "/echo",
            post(|body: String| async move { format!("got: {body}") }),
        );
        let addr = start_test_server(app).await;
        let fwd = Forwarder::new(addr);

        let meta = test_meta("POST", "/echo");
        let result = fwd.forward(&meta, Bytes::from("payload")).await;
        let (meta, body, _conn) = collect_success(result).await;

        assert_eq!(meta.status, 200);
        assert_eq!(body, b"got: payload");
        Ok(())
    }

    #[tokio::test]
    async fn forward_preserves_response_headers() -> TestResult {
        let app = Router::new().route(
            "/headers",
            get(|| async {
                (
                    [("x-custom", "test-val"), ("content-type", "text/plain")],
                    "ok",
                )
            }),
        );
        let addr = start_test_server(app).await;
        let fwd = Forwarder::new(addr);

        let meta = test_meta("GET", "/headers");
        let result = fwd.forward(&meta, Bytes::new()).await;
        let (meta, _, _conn) = collect_success(result).await;

        assert_eq!(meta.status, 200);
        assert_eq!(
            meta.headers.get("x-custom").map(Vec::as_slice),
            Some(["test-val".to_string()].as_slice())
        );
        Ok(())
    }

    #[tokio::test]
    async fn forward_strips_hop_by_hop_response_headers() -> TestResult {
        let app = Router::new().route(
            "/hop",
            get(|| async { ([("connection", "keep-alive"), ("x-real", "value")], "ok") }),
        );
        let addr = start_test_server(app).await;
        let fwd = Forwarder::new(addr);

        let meta = test_meta("GET", "/hop");
        let result = fwd.forward(&meta, Bytes::new()).await;
        let (meta, _, _conn) = collect_success(result).await;

        assert!(!meta.headers.contains_key("connection"));
        assert!(meta.headers.contains_key("x-real"));
        Ok(())
    }

    #[tokio::test]
    async fn forward_returns_404() -> TestResult {
        let app = Router::new();
        let addr = start_test_server(app).await;
        let fwd = Forwarder::new(addr);

        let meta = test_meta("GET", "/missing");
        let result = fwd.forward(&meta, Bytes::new()).await;
        let (meta, _, _conn) = collect_success(result).await;

        assert_eq!(meta.status, 404);
        Ok(())
    }

    #[tokio::test]
    async fn forward_unreachable_returns_502() -> TestResult {
        let fwd = Forwarder::new("127.0.0.1:1".to_string());

        let meta = test_meta("GET", "/");
        let result = fwd.forward(&meta, Bytes::new()).await;

        match result {
            ForwardResult::LocalError { meta, body } => {
                assert_eq!(meta.status, 502);
                assert!(String::from_utf8_lossy(&body).contains("local service error"));
            }
            ForwardResult::Success { .. } => {
                panic!("expected LocalError for unreachable host");
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn forward_sends_request_headers() -> TestResult {
        let app = Router::new().route(
            "/check",
            get(|headers: axum::http::HeaderMap| async move {
                headers
                    .get("x-test")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("missing")
                    .to_string()
            }),
        );
        let addr = start_test_server(app).await;
        let fwd = Forwarder::new(addr);

        let mut meta = test_meta("GET", "/check");
        meta.headers.insert("x-test".into(), vec!["present".into()]);

        let result = fwd.forward(&meta, Bytes::new()).await;
        let (resp, body, _conn) = collect_success(result).await;

        assert_eq!(resp.status, 200);
        assert_eq!(body, b"present");
        Ok(())
    }

    #[tokio::test]
    async fn forward_strips_accept_encoding() -> TestResult {
        let app = Router::new().route(
            "/check",
            get(|headers: axum::http::HeaderMap| async move {
                headers
                    .get("accept-encoding")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("missing")
                    .to_string()
            }),
        );
        let addr = start_test_server(app).await;
        let fwd = Forwarder::new(addr);

        let mut meta = test_meta("GET", "/check");
        meta.headers
            .insert("accept-encoding".into(), vec!["gzip, br, zstd".into()]);

        let result = fwd.forward(&meta, Bytes::new()).await;
        let (resp, body, _conn) = collect_success(result).await;

        assert_eq!(resp.status, 200);
        assert_eq!(body, b"missing");
        Ok(())
    }

    #[tokio::test]
    async fn connection_pool_reuses_connections() -> TestResult {
        let app = Router::new().route("/ok", get(|| async { "ok" }));
        let addr = start_test_server(app).await;
        let fwd = Forwarder::new(addr);

        let meta = test_meta("GET", "/ok");
        let result = fwd.forward(&meta, Bytes::new()).await;
        let (meta, _, conn) = collect_success(result).await;
        assert_eq!(meta.status, 200);
        fwd.release(conn);

        assert_eq!(fwd.pool.lock().unwrap().len(), 1);

        let meta = test_meta("GET", "/ok");
        let result = fwd.forward(&meta, Bytes::new()).await;
        let (meta, _, _conn) = collect_success(result).await;
        assert_eq!(meta.status, 200);

        assert_eq!(fwd.pool.lock().unwrap().len(), 0);
        Ok(())
    }

    async fn ws_echo_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
        ws.on_upgrade(ws_echo)
    }

    async fn ws_echo(mut socket: WebSocket) {
        while let Some(Ok(msg)) = socket.next().await {
            if matches!(msg, Message::Close(_)) {
                break;
            }
            if matches!(msg, Message::Text(_) | Message::Binary(_))
                && socket.send(msg).await.is_err()
            {
                break;
            }
        }
    }

    #[tokio::test]
    async fn forward_upgrade_websocket() -> TestResult {
        let app = Router::new().route("/ws", get(ws_echo_handler));
        let addr = start_test_server(app).await;
        let fwd = Forwarder::new(addr);

        let meta = upgrade_meta("/ws");
        let result = fwd.forward_upgrade(&meta).await?;

        match result {
            ForwardUpgradeResult::Upgraded(u) => {
                assert_eq!(u.meta.status, 101);
                assert!(u.meta.headers.contains_key("upgrade"));
                assert!(u.meta.headers.contains_key("connection"));
            }
            ForwardUpgradeResult::Rejected(meta, _) => {
                panic!(
                    "expected upgrade, got rejection with status {}",
                    meta.status
                );
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn forward_upgrade_rejected_by_non_ws_endpoint() -> TestResult {
        let app = Router::new().route("/not-ws", get(|| async { "not a websocket" }));
        let addr = start_test_server(app).await;
        let fwd = Forwarder::new(addr);

        let meta = upgrade_meta("/not-ws");
        let result = fwd.forward_upgrade(&meta).await?;

        match result {
            ForwardUpgradeResult::Rejected(meta, _) => {
                assert_ne!(meta.status, 101);
            }
            ForwardUpgradeResult::Upgraded(_) => {
                panic!("expected rejection from non-ws endpoint");
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn forward_upgrade_unreachable_returns_error() {
        let fwd = Forwarder::new("127.0.0.1:1".to_string());
        let meta = upgrade_meta("/ws");

        let result = fwd.forward_upgrade(&meta).await;
        assert!(result.is_err());
    }
}
