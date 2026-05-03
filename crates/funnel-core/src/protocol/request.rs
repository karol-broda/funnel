use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMeta {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMeta {
    pub status: u16,
    pub headers: HashMap<String, Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_meta_roundtrip() {
        let meta = RequestMeta {
            method: "POST".into(),
            path: "/api/data".into(),
            headers: {
                let mut h = HashMap::new();
                h.insert("content-type".into(), vec!["application/json".into()]);
                h
            },
        };

        let encoded = rmp_serde::to_vec_named(&meta).unwrap();
        let decoded: RequestMeta = rmp_serde::from_slice(&encoded).unwrap();
        assert_eq!(decoded.method, "POST");
        assert_eq!(decoded.path, "/api/data");
    }

    #[test]
    fn response_meta_roundtrip() {
        let meta = ResponseMeta {
            status: 200,
            headers: HashMap::new(),
        };

        let encoded = rmp_serde::to_vec_named(&meta).unwrap();
        let decoded: ResponseMeta = rmp_serde::from_slice(&encoded).unwrap();
        assert_eq!(decoded.status, 200);
    }
}
