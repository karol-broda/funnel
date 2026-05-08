use std::collections::HashMap;
use std::hash::BuildHasher;

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum ProtocolTypeError {
    #[error("invalid http method: {0}")]
    InvalidMethod(String),

    #[error("invalid http status code: {0}")]
    InvalidStatus(u16),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMeta {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, Vec<String>>,
}

impl RequestMeta {
    pub fn http_method(&self) -> Result<http::Method, ProtocolTypeError> {
        self.method
            .parse()
            .map_err(|_| ProtocolTypeError::InvalidMethod(self.method.clone()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMeta {
    pub status: u16,
    pub headers: HashMap<String, Vec<String>>,
}

impl ResponseMeta {
    pub fn http_status(&self) -> Result<http::StatusCode, ProtocolTypeError> {
        http::StatusCode::from_u16(self.status)
            .map_err(|_| ProtocolTypeError::InvalidStatus(self.status))
    }
}

/// convert a protocol header map into an http `HeaderMap`, skipping invalid entries.
pub fn to_header_map<S: BuildHasher>(
    headers: &HashMap<String, Vec<String>, S>,
) -> http::HeaderMap {
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
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn request_meta_roundtrip() -> TestResult {
        let meta = RequestMeta {
            method: "POST".into(),
            path: "/api/data".into(),
            headers: {
                let mut h = HashMap::new();
                h.insert("content-type".into(), vec!["application/json".into()]);
                h
            },
        };

        let encoded = rmp_serde::to_vec_named(&meta)?;
        let decoded: RequestMeta = rmp_serde::from_slice(&encoded)?;
        assert_eq!(decoded.method, "POST");
        assert_eq!(decoded.path, "/api/data");
        Ok(())
    }

    #[test]
    fn response_meta_roundtrip() -> TestResult {
        let meta = ResponseMeta {
            status: 200,
            headers: HashMap::new(),
        };

        let encoded = rmp_serde::to_vec_named(&meta)?;
        let decoded: ResponseMeta = rmp_serde::from_slice(&encoded)?;
        assert_eq!(decoded.status, 200);
        Ok(())
    }

    #[test]
    fn valid_http_method() {
        let meta = RequestMeta {
            method: "GET".into(),
            path: "/".into(),
            headers: HashMap::new(),
        };
        assert_eq!(meta.http_method().ok(), Some(http::Method::GET));
    }

    #[test]
    fn invalid_http_method() {
        let meta = RequestMeta {
            method: "".into(),
            path: "/".into(),
            headers: HashMap::new(),
        };
        assert!(meta.http_method().is_err());
    }

    #[test]
    fn valid_http_status() {
        let meta = ResponseMeta {
            status: 404,
            headers: HashMap::new(),
        };
        assert_eq!(
            meta.http_status().ok(),
            Some(http::StatusCode::NOT_FOUND)
        );
    }

    #[test]
    fn invalid_http_status() {
        let meta = ResponseMeta {
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
        assert_eq!(map.get("content-type").and_then(|v| v.to_str().ok()), Some("text/html"));
        assert_eq!(map.get_all("x-custom").iter().count(), 2);
    }

    #[test]
    fn to_header_map_skips_invalid() {
        let mut headers = HashMap::new();
        headers.insert("valid".into(), vec!["ok".into()]);
        headers.insert("".into(), vec!["bad name".into()]);

        let map = to_header_map(&headers);
        assert_eq!(map.len(), 1);
    }
}
