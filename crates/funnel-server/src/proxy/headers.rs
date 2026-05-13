use std::collections::HashMap;
use std::net::SocketAddr;

use axum::http::HeaderMap;

/// copy request headers and add standard proxy headers (X-Forwarded-For, etc).
pub fn prepare_forwarding_headers(
    original: &HeaderMap,
    host: &str,
    remote_addr: SocketAddr,
    is_tls: bool,
) -> HashMap<String, Vec<String>> {
    let mut headers: HashMap<String, Vec<String>> = HashMap::new();

    for (name, value) in original {
        let key = name.as_str().to_string();
        let val = value.to_str().unwrap_or("").to_string();
        headers.entry(key).or_default().push(val);
    }

    let client_ip = remote_addr.ip().to_string();

    // append to existing X-Forwarded-For or create new
    match headers.get("x-forwarded-for") {
        Some(existing) if !existing.is_empty() => {
            let all = existing.join(", ");
            let combined = format!("{all}, {client_ip}");
            headers.insert("x-forwarded-for".to_string(), vec![combined]);
        }
        _ => {
            headers.insert("x-forwarded-for".to_string(), vec![client_ip.clone()]);
        }
    }

    if !host.is_empty() {
        headers.insert("x-forwarded-host".to_string(), vec![host.to_string()]);
    }

    let proto = if is_tls { "https" } else { "http" };
    headers.insert("x-forwarded-proto".to_string(), vec![proto.to_string()]);
    headers.insert("x-real-ip".to_string(), vec![client_ip]);

    headers
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use std::net::{IpAddr, Ipv4Addr};

    fn test_addr() -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 54321)
    }

    #[test]
    fn adds_forwarding_headers() {
        let headers = HeaderMap::new();
        let result =
            prepare_forwarding_headers(&headers, "my-tunnel.example.com", test_addr(), false);

        assert_eq!(result["x-forwarded-for"], vec!["192.168.1.100"]);
        assert_eq!(result["x-forwarded-host"], vec!["my-tunnel.example.com"]);
        assert_eq!(result["x-forwarded-proto"], vec!["http"]);
        assert_eq!(result["x-real-ip"], vec!["192.168.1.100"]);
    }

    #[test]
    fn appends_to_existing_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("10.0.0.1"));

        let result = prepare_forwarding_headers(&headers, "t.example.com", test_addr(), false);

        assert_eq!(result["x-forwarded-for"], vec!["10.0.0.1, 192.168.1.100"]);
    }

    #[test]
    fn appends_to_multi_value_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.append("x-forwarded-for", HeaderValue::from_static("10.0.0.1"));
        headers.append("x-forwarded-for", HeaderValue::from_static("172.16.0.1"));

        let result = prepare_forwarding_headers(&headers, "t.example.com", test_addr(), false);

        assert_eq!(
            result["x-forwarded-for"],
            vec!["10.0.0.1, 172.16.0.1, 192.168.1.100"]
        );
    }

    #[test]
    fn sets_https_proto_when_tls() {
        let headers = HeaderMap::new();
        let result = prepare_forwarding_headers(&headers, "t.example.com", test_addr(), true);

        assert_eq!(result["x-forwarded-proto"], vec!["https"]);
    }

    #[test]
    fn copies_original_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("authorization", HeaderValue::from_static("Bearer token123"));

        let result = prepare_forwarding_headers(&headers, "t.example.com", test_addr(), false);

        assert_eq!(result["content-type"], vec!["application/json"]);
        assert_eq!(result["authorization"], vec!["Bearer token123"]);
    }
}
