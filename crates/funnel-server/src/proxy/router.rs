use std::sync::Arc;

use axum::body::Body;
use axum::extract::{ConnectInfo, Host, State};
use axum::http::{Request, Response, StatusCode};

use funnel_core::protocol::RequestPayload;
use funnel_core::tunnel::TunnelId;

use crate::app::AppState;
use super::headers::prepare_forwarding_headers;

const MAX_BODY_SIZE: usize = 10 * 1024 * 1024; // 10 mb

/// axum fallback handler that routes requests based on subdomain.
/// requests to `{tunnel_id}.{base_domain}` are forwarded through the matching tunnel.
pub async fn handle_tunnel_request(
    State(state): State<Arc<AppState>>,
    Host(host): Host,
    ConnectInfo(remote_addr): ConnectInfo<std::net::SocketAddr>,
    request: Request<Body>,
) -> Response<Body> {
    let subdomain = match extract_subdomain(&host) {
        Some(s) => s,
        None => return not_found("tunnel not found"),
    };

    let tunnel_id = match TunnelId::new(subdomain) {
        Ok(id) => id,
        Err(_) => return not_found("tunnel not found"),
    };

    let tunnel = match state.tunnels.get(&tunnel_id) {
        Some(t) => t,
        None => return not_found("tunnel not found"),
    };

    let method = request.method().to_string();
    let path = request.uri().to_string();
    let is_tls = false; // will be determined by tls termination layer later

    let headers = prepare_forwarding_headers(request.headers(), &host, remote_addr, is_tls);

    let body_bytes = match axum::body::to_bytes(request.into_body(), MAX_BODY_SIZE).await {
        Ok(b) => b.to_vec(),
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "request body too large"),
    };

    let payload = RequestPayload {
        method,
        path,
        headers,
        body: body_bytes,
    };

    match tunnel.send_request(payload).await {
        Ok(response) => build_response(response),
        Err(crate::tunnel::connection::SendError::Timeout) => {
            error_response(StatusCode::GATEWAY_TIMEOUT, "request timed out")
        }
        Err(crate::tunnel::connection::SendError::TunnelClosed) => {
            error_response(StatusCode::BAD_GATEWAY, "tunnel connection lost")
        }
    }
}

/// extract the first subdomain label from a host string.
/// `"my-tunnel.example.com"` -> `Some("my-tunnel")`
/// `"example.com"` -> `None` (no subdomain)
/// `"localhost:8080"` -> `None`
fn extract_subdomain(host: &str) -> Option<&str> {
    // strip port if present
    let host = host.split(':').next().unwrap_or(host);

    let dot = host.find('.')?;
    if dot == 0 {
        return None;
    }

    let subdomain = &host[..dot];
    let rest = &host[dot + 1..];

    // need at least one more dot for it to be a real subdomain
    // (subdomain.domain.tld), not just (domain.tld)
    if rest.contains('.') {
        Some(subdomain)
    } else {
        None
    }
}

fn build_response(payload: funnel_core::protocol::ResponsePayload) -> Response<Body> {
    let mut builder = Response::builder().status(payload.status);

    if let Some(headers) = builder.headers_mut() {
        for (name, values) in &payload.headers {
            for value in values {
                if let (Ok(name), Ok(value)) = (
                    axum::http::HeaderName::try_from(name.as_str()),
                    axum::http::HeaderValue::from_str(value),
                ) {
                    headers.append(name, value);
                }
            }
        }
    }

    builder
        .body(Body::from(payload.body))
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
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_subdomain_basic() {
        assert_eq!(extract_subdomain("my-tunnel.example.com"), Some("my-tunnel"));
        assert_eq!(extract_subdomain("abc.example.com"), Some("abc"));
    }

    #[test]
    fn extract_subdomain_with_port() {
        assert_eq!(extract_subdomain("my-tunnel.example.com:8080"), Some("my-tunnel"));
    }

    #[test]
    fn extract_subdomain_no_subdomain() {
        assert_eq!(extract_subdomain("example.com"), None);
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
        // only extracts the first label
        assert_eq!(extract_subdomain("a.b.example.com"), Some("a"));
    }
}
