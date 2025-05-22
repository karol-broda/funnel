use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::io;
use tokio_util::io::ReaderStream;

use funnel_core::protocol::request::{self as proto, HttpRequest, HttpResponse};
use funnel_core::tunnel::id::TunnelId;

use super::headers::prepare_forwarding_headers;
use crate::app::AppState;
use crate::tunnel::access::AccessDenied;
use crate::tunnel::connection::{ActiveTunnel, CountedRecvStream, SendError};

fn is_upgrade_request(req: &Request<Body>) -> bool {
    req.headers()
        .get("connection")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("upgrade"))
}

/// axum fallback handler that routes requests based on subdomain.
/// requests to `{tunnel_id}.{base_domain}` are forwarded through the matching tunnel.
pub async fn handle_tunnel_request(
    State(state): State<Arc<AppState>>,
    mut request: Request<Body>,
) -> Response<Body> {
    let host = request
        .headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let tunnel_id = request
        .extensions()
        .get::<TunnelId>()
        .cloned()
        .or_else(|| extract_subdomain(&host).and_then(|s| TunnelId::new(s).ok()));

    let Some(tunnel_id) = tunnel_id else {
        return not_found("tunnel not found");
    };

    let Some(tunnel) = state.tunnels.get(&tunnel_id) else {
        return not_found("tunnel not found");
    };

    let fallback_addr = std::net::SocketAddr::from(([0, 0, 0, 0], 0));
    let remote_addr = request
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map_or(fallback_addr, |ci| ci.0);

    if let Err(denied) = tunnel.check_access(request.headers(), remote_addr.ip()) {
        return access_denied_response(denied);
    }

    if is_upgrade_request(&request) {
        return handle_upgrade(&mut request, &host, remote_addr, &state, &tunnel).await;
    }

    let method = request.method().to_string();
    let path = request.uri().to_string();

    let headers = prepare_forwarding_headers(request.headers(), &host, remote_addr, state.is_tls);

    let meta = HttpRequest {
        tunnel_id: tunnel_id.clone(),
        remote_addr: remote_addr.to_string(),
        method,
        path,
        headers,
        upgrade: false,
    };

    let body = request.into_body();

    match tunnel.send_request(meta, body).await {
        Ok((resp_meta, recv_stream)) => build_response(&resp_meta, recv_stream),
        Err(SendError::Timeout) => error_response(StatusCode::GATEWAY_TIMEOUT, "request timed out"),
        Err(SendError::ReadBody(e)) => {
            tracing::debug!(error = %e, "failed to read request body");
            error_response(StatusCode::BAD_REQUEST, "failed to read request body")
        }
        Err(e) => {
            tracing::debug!(error = %e, "tunnel request failed");
            error_response(StatusCode::BAD_GATEWAY, "tunnel error")
        }
    }
}

async fn handle_upgrade(
    request: &mut Request<Body>,
    host: &str,
    remote_addr: std::net::SocketAddr,
    state: &AppState,
    tunnel: &ActiveTunnel,
) -> Response<Body> {
    let method = request.method().to_string();
    let path = request.uri().to_string();

    let headers = prepare_forwarding_headers(request.headers(), host, remote_addr, state.is_tls);

    let meta = HttpRequest {
        tunnel_id: tunnel.id().clone(),
        remote_addr: remote_addr.to_string(),
        method,
        path,
        headers,
        upgrade: true,
    };

    // capture the upgrade future before we consume anything
    let on_upgrade = hyper::upgrade::on(request);

    let (resp_meta, quic_send, quic_recv) = match tunnel.send_upgrade_request(meta).await {
        Ok(v) => v,
        Err(SendError::Timeout) => {
            return error_response(StatusCode::GATEWAY_TIMEOUT, "request timed out");
        }
        Err(e) => {
            tracing::debug!(error = %e, "upgrade tunnel request failed");
            return error_response(StatusCode::BAD_GATEWAY, "tunnel error");
        }
    };

    let status = resp_meta.http_status().unwrap_or(StatusCode::BAD_GATEWAY);

    if status != StatusCode::SWITCHING_PROTOCOLS {
        let mut builder = Response::builder().status(status);
        if let Some(h) = builder.headers_mut() {
            *h = proto::to_header_map(&resp_meta.headers);
        }
        return builder.body(Body::empty()).unwrap_or_else(|_| {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "internal error")
        });
    }

    let mut builder = Response::builder().status(StatusCode::SWITCHING_PROTOCOLS);
    if let Some(h) = builder.headers_mut() {
        *h = proto::to_header_map(&resp_meta.headers);
    }

    tokio::spawn(async move {
        let upgraded = match on_upgrade.await {
            Ok(u) => u,
            Err(e) => {
                tracing::debug!(error = %e, "upgrade handshake failed");
                return;
            }
        };

        let upgraded_io = TokioIo::new(upgraded);
        let (mut browser_read, mut browser_write) = io::split(upgraded_io);
        let (mut quic_send, mut quic_recv) = (quic_send, quic_recv);

        let client_to_quic = io::copy(&mut browser_read, &mut quic_send);
        let quic_to_client = io::copy(&mut quic_recv, &mut browser_write);

        tokio::select! {
            r = client_to_quic => {
                if let Err(e) = r {
                    tracing::debug!(error = %e, "browser to quic copy ended");
                }
            }
            r = quic_to_client => {
                if let Err(e) = r {
                    tracing::debug!(error = %e, "quic to browser copy ended");
                }
            }
        }
    });

    builder
        .body(Body::empty())
        .unwrap_or_else(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "internal error"))
}

pub fn extract_subdomain(host: &str) -> Option<&str> {
    let host = host.split(':').next().unwrap_or(host);

    let dot = host.find('.')?;
    if dot == 0 {
        return None;
    }

    let subdomain = &host[..dot];
    let rest = &host[dot + 1..];

    if rest.is_empty() {
        None
    } else {
        Some(subdomain)
    }
}

fn build_response(meta: &HttpResponse, recv: CountedRecvStream) -> Response<Body> {
    let status = meta.http_status().unwrap_or(StatusCode::BAD_GATEWAY);
    let mut builder = Response::builder().status(status);

    if let Some(headers) = builder.headers_mut() {
        *headers = proto::to_header_map(&meta.headers);
    }

    let body_stream = ReaderStream::new(recv);
    let body = Body::from_stream(body_stream);

    builder
        .body(body)
        .unwrap_or_else(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "internal error"))
}

fn not_found(msg: &str) -> Response<Body> {
    error_response(StatusCode::NOT_FOUND, msg)
}

fn access_denied_response(denied: AccessDenied) -> Response<Body> {
    match denied {
        AccessDenied::Expired => error_response(StatusCode::GONE, "tunnel expired"),
        AccessDenied::IpForbidden => error_response(StatusCode::FORBIDDEN, "access denied"),
        AccessDenied::ProxyAuthRequired => Response::builder()
            .status(StatusCode::PROXY_AUTHENTICATION_REQUIRED)
            .header("content-type", "text/plain")
            .header("proxy-authenticate", "Basic realm=\"funnel\"")
            .body(Body::from("proxy authentication required"))
            .unwrap_or_else(|_| Response::new(Body::from("proxy authentication required"))),
    }
}

fn error_response(status: StatusCode, msg: &str) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("content-type", "text/plain")
        .body(Body::from(msg.to_string()))
        .unwrap_or_else(|_| Response::new(Body::from(msg.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_subdomain_basic() {
        assert_eq!(
            extract_subdomain("my-tunnel.example.com"),
            Some("my-tunnel")
        );
        assert_eq!(extract_subdomain("abc.example.com"), Some("abc"));
    }

    #[test]
    fn extract_subdomain_with_port() {
        assert_eq!(
            extract_subdomain("my-tunnel.example.com:8080"),
            Some("my-tunnel")
        );
    }

    #[test]
    fn extract_subdomain_single_label() {
        assert_eq!(extract_subdomain("abc.localhost"), Some("abc"));
        assert_eq!(extract_subdomain("abc.localhost:8080"), Some("abc"));
    }

    #[test]
    fn extract_subdomain_no_subdomain() {
        assert_eq!(extract_subdomain("localhost"), None);
        assert_eq!(extract_subdomain("localhost:8080"), None);
    }

    #[test]
    fn extract_subdomain_empty_or_dot_prefix() {
        assert_eq!(extract_subdomain(""), None);
        assert_eq!(extract_subdomain(".example.com"), None);
    }

    #[test]
    fn extract_subdomain_deep_nesting() {
        assert_eq!(extract_subdomain("a.b.example.com"), Some("a"));
    }

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn detects_upgrade_request() -> TestResult {
        let req = Request::builder()
            .header("connection", "Upgrade")
            .header("upgrade", "websocket")
            .body(Body::empty())?;
        assert!(is_upgrade_request(&req));
        Ok(())
    }

    #[test]
    fn normal_request_is_not_upgrade() -> TestResult {
        let req = Request::builder().body(Body::empty())?;
        assert!(!is_upgrade_request(&req));
        Ok(())
    }
}
