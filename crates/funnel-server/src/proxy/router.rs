use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, Response, StatusCode};
use tokio_util::io::ReaderStream;

use funnel_core::protocol::request::{self as proto, RequestMeta, ResponseMeta};
use funnel_core::tunnel::id::TunnelId;

use super::headers::prepare_forwarding_headers;
use crate::app::AppState;
use crate::tunnel::connection::{CountedRecvStream, SendError};

/// axum fallback handler that routes requests based on subdomain.
/// requests to `{tunnel_id}.{base_domain}` are forwarded through the matching tunnel.
pub async fn handle_tunnel_request(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
) -> Response<Body> {
    let host = request
        .headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let subdomain = match extract_subdomain(host) {
        Some(s) => s.to_string(),
        None => return not_found("tunnel not found"),
    };

    let Ok(tunnel_id) = TunnelId::new(&subdomain) else {
        return not_found("tunnel not found");
    };

    let Some(tunnel) = state.tunnels.get(&tunnel_id) else {
        return not_found("tunnel not found");
    };

    let method = request.method().to_string();
    let path = request.uri().to_string();

    let fallback_addr = std::net::SocketAddr::from(([0, 0, 0, 0], 0));
    let remote_addr = request
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map_or(fallback_addr, |ci| ci.0);

    let headers = prepare_forwarding_headers(request.headers(), host, remote_addr, state.is_tls);

    let meta = RequestMeta {
        method,
        path,
        headers,
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

fn extract_subdomain(host: &str) -> Option<&str> {
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

fn build_response(meta: &ResponseMeta, recv: CountedRecvStream) -> Response<Body> {
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
}
