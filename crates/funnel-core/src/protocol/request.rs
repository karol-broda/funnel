use std::collections::HashMap;
use std::hash::BuildHasher;

use serde::{Deserialize, Serialize};

use crate::tunnel::id::TunnelId;

#[derive(Debug, thiserror::Error)]
pub enum ProtocolTypeError {
    #[error("invalid http method: {0}")]
    InvalidMethod(String),

    #[error("invalid http status code: {0}")]
    InvalidStatus(u16),
}

/// metadata frame for stream tunnel data streams (TCP, TLS passthrough).
/// sent by the server at the start of each QUIC bidirectional stream.
/// after this frame, the stream is a raw bidirectional byte pipe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamHeader {
    pub tunnel_id: TunnelId,
    pub remote_addr: String,
    pub server_port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sni: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequest {
    pub tunnel_id: TunnelId,
    pub remote_addr: String,
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub upgrade: bool,
}

impl HttpRequest {
    pub fn http_method(&self) -> Result<http::Method, ProtocolTypeError> {
        self.method
            .parse()
            .map_err(|_| ProtocolTypeError::InvalidMethod(self.method.clone()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, Vec<String>>,
}

impl HttpResponse {
    pub fn http_status(&self) -> Result<http::StatusCode, ProtocolTypeError> {
        http::StatusCode::from_u16(self.status)
            .map_err(|_| ProtocolTypeError::InvalidStatus(self.status))
    }
}

/// metadata frame sent by the server at the start of each data stream.
/// the client deserializes this to determine whether it's handling an HTTP
/// request or a raw stream connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DataHeader {
    #[serde(rename = "http")]
    Http(HttpRequest),
    #[serde(rename = "stream")]
    Stream(StreamHeader),
}

/// convert a protocol header map into an http `HeaderMap`, skipping invalid entries.
pub fn to_header_map<S: BuildHasher>(headers: &HashMap<String, Vec<String>, S>) -> http::HeaderMap {
    let mut map = http::HeaderMap::new();
    for (name, values) in headers {
        for value in values {
            if let (Ok(name), Ok(value)) = (
                http::HeaderName::try_from(name.as_str()),
                http::HeaderValue::from_str(value),
            ) {
                map.append(name, value);
            }
        }
    }
    map
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn http_request_roundtrip() -> TestResult {
        let meta = HttpRequest {
            tunnel_id: TunnelId::new("test").unwrap(),
            remote_addr: "127.0.0.1:0".into(),
            method: "POST".into(),
            path: "/api/data".into(),
            headers: {
                let mut h = HashMap::new();
                h.insert("content-type".into(), vec!["application/json".into()]);
                h
            },
            upgrade: false,
        };

        let encoded = rmp_serde::to_vec_named(&meta)?;
        let decoded: HttpRequest = rmp_serde::from_slice(&encoded)?;
        assert_eq!(decoded.method, "POST");
        assert_eq!(decoded.path, "/api/data");
        Ok(())
    }

    #[test]
    fn http_response_roundtrip() -> TestResult {
        let meta = HttpResponse {
            status: 200,
            headers: HashMap::new(),
        };

        let encoded = rmp_serde::to_vec_named(&meta)?;
        let decoded: HttpResponse = rmp_serde::from_slice(&encoded)?;
        assert_eq!(decoded.status, 200);
        Ok(())
    }

    #[test]
    fn valid_http_method() {
        let meta = HttpRequest {
            tunnel_id: TunnelId::new("test").unwrap(),
            remote_addr: "127.0.0.1:0".into(),
            method: "GET".into(),
            path: "/".into(),
            headers: HashMap::new(),
            upgrade: false,
        };
        assert_eq!(meta.http_method().ok(), Some(http::Method::GET));
    }

    #[test]
    fn invalid_http_method() {
        let meta = HttpRequest {
            tunnel_id: TunnelId::new("test").unwrap(),
            remote_addr: "127.0.0.1:0".into(),
            method: String::new(),
            path: "/".into(),
            headers: HashMap::new(),
            upgrade: false,
        };
        assert!(meta.http_method().is_err());
    }

    #[test]
    fn valid_http_status() {
        let meta = HttpResponse {
            status: 404,
            headers: HashMap::new(),
        };
        assert_eq!(meta.http_status().ok(), Some(http::StatusCode::NOT_FOUND));
    }

    #[test]
    fn invalid_http_status() {
        let meta = HttpResponse {
            status: 9999,
            headers: HashMap::new(),
        };
        assert!(meta.http_status().is_err());
    }

    #[test]
    fn to_header_map_converts_valid_entries() {
        let mut headers = HashMap::new();
        headers.insert("content-type".into(), vec!["text/html".into()]);
        headers.insert("x-custom".into(), vec!["a".into(), "b".into()]);

        let map = to_header_map(&headers);
        assert_eq!(
            map.get("content-type").and_then(|v| v.to_str().ok()),
            Some("text/html")
        );
        assert_eq!(map.get_all("x-custom").iter().count(), 2);
    }

    #[test]
    fn stream_header_roundtrip() -> TestResult {
        let header = StreamHeader {
            tunnel_id: TunnelId::new("my-db").unwrap(),
            remote_addr: "203.0.113.42:9999".into(),
            server_port: 15432,
            sni: None,
        };

        let encoded = rmp_serde::to_vec_named(&header)?;
        let decoded: StreamHeader = rmp_serde::from_slice(&encoded)?;
        assert_eq!(decoded.tunnel_id.as_ref(), "my-db");
        assert_eq!(decoded.server_port, 15432);
        assert!(decoded.sni.is_none());
        Ok(())
    }

    #[test]
    fn stream_header_with_sni_roundtrip() -> TestResult {
        let header = StreamHeader {
            tunnel_id: TunnelId::new("secure").unwrap(),
            remote_addr: "10.0.0.1:443".into(),
            server_port: 443,
            sni: Some("app.example.com".into()),
        };

        let encoded = rmp_serde::to_vec_named(&header)?;
        let decoded: StreamHeader = rmp_serde::from_slice(&encoded)?;
        assert_eq!(decoded.sni.as_deref(), Some("app.example.com"));
        Ok(())
    }

    #[test]
    fn data_header_http_roundtrip() -> TestResult {
        let header = DataHeader::Http(HttpRequest {
            tunnel_id: TunnelId::new("test").unwrap(),
            remote_addr: "127.0.0.1:0".into(),
            method: "GET".into(),
            path: "/".into(),
            headers: HashMap::new(),
            upgrade: false,
        });

        let encoded = rmp_serde::to_vec_named(&header)?;
        let decoded: DataHeader = rmp_serde::from_slice(&encoded)?;
        assert!(matches!(decoded, DataHeader::Http(_)));
        Ok(())
    }

    #[test]
    fn data_header_stream_roundtrip() -> TestResult {
        let header = DataHeader::Stream(StreamHeader {
            tunnel_id: TunnelId::new("my-db").unwrap(),
            remote_addr: "10.0.0.1:5432".into(),
            server_port: 15432,
            sni: None,
        });

        let encoded = rmp_serde::to_vec_named(&header)?;
        let decoded: DataHeader = rmp_serde::from_slice(&encoded)?;
        assert!(matches!(decoded, DataHeader::Stream(_)));
        Ok(())
    }

    #[test]
    fn to_header_map_skips_invalid() {
        let mut headers = HashMap::new();
        headers.insert("valid".into(), vec!["ok".into()]);
        headers.insert(String::new(), vec!["bad name".into()]);

        let map = to_header_map(&headers);
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn data_header_unknown_type_fails_deserialization() {
        // manually craft a msgpack map with an unknown type tag
        let unknown = serde_json::json!({
            "type": "unknown",
            "tunnel_id": "test",
            "remote_addr": "1.2.3.4:80",
        });
        let bytes = rmp_serde::to_vec_named(&unknown).unwrap();
        let result = rmp_serde::from_slice::<DataHeader>(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn data_header_missing_type_fails() {
        // a msgpack map without a "type" field
        let no_type = serde_json::json!({
            "tunnel_id": "test",
            "remote_addr": "1.2.3.4:80",
            "method": "GET",
            "path": "/",
            "headers": {},
        });
        let bytes = rmp_serde::to_vec_named(&no_type).unwrap();
        let result = rmp_serde::from_slice::<DataHeader>(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn data_header_http_preserves_fields() -> TestResult {
        let original = HttpRequest {
            tunnel_id: TunnelId::new("my-app").unwrap(),
            remote_addr: "203.0.113.42:9999".into(),
            method: "POST".into(),
            path: "/api/data?key=value".into(),
            headers: {
                let mut h = HashMap::new();
                h.insert("content-type".into(), vec!["application/json".into()]);
                h.insert("x-multi".into(), vec!["a".into(), "b".into()]);
                h
            },
            upgrade: true,
        };

        let header = DataHeader::Http(original);
        let encoded = rmp_serde::to_vec_named(&header)?;
        let decoded: DataHeader = rmp_serde::from_slice(&encoded)?;

        match decoded {
            DataHeader::Http(req) => {
                assert_eq!(req.tunnel_id.as_ref(), "my-app");
                assert_eq!(req.method, "POST");
                assert_eq!(req.path, "/api/data?key=value");
                assert!(req.upgrade);
                assert_eq!(req.headers.get("x-multi").map(Vec::len), Some(2));
            }
            DataHeader::Stream(_) => panic!("expected Http variant"),
        }
        Ok(())
    }

    #[test]
    fn stream_header_empty_tunnel_id_fails() {
        let result = TunnelId::new("");
        assert!(result.is_err());
    }

    #[test]
    fn stream_header_tunnel_id_too_short() {
        let result = TunnelId::new("ab");
        assert!(result.is_err());
    }
}
