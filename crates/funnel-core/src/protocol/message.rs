use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::tunnel::TunnelId;

/// HTTP request payload forwarded through the tunnel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestPayload {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, Vec<String>>,
    #[serde(with = "base64_bytes")]
    pub body: Vec<u8>,
}

/// HTTP response payload returned through the tunnel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsePayload {
    pub status: u16,
    pub headers: HashMap<String, Vec<String>>,
    #[serde(with = "base64_bytes")]
    pub body: Vec<u8>,
}

/// A message sent over the WebSocket tunnel connection.
///
/// Uses a tagged enum (`"type"` field) for clean pattern matching
/// instead of stringly-typed message kinds.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TunnelMessage {
    Request {
        tunnel_id: TunnelId,
        request_id: Uuid,
        payload: RequestPayload,
    },
    Response {
        request_id: Uuid,
        payload: ResponsePayload,
    },
    Ping,
    Pong,
    RequestCancel {
        request_id: Uuid,
    },
}

/// Serde helper for encoding `Vec<u8>` as base64 strings in JSON.
mod base64_bytes {
    use base64::prelude::*;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&BASE64_STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        BASE64_STANDARD.decode(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_request() {
        let msg = TunnelMessage::Request {
            tunnel_id: TunnelId::new("my-tunnel").unwrap(),
            request_id: Uuid::nil(),
            payload: RequestPayload {
                method: "GET".into(),
                path: "/hello".into(),
                headers: HashMap::new(),
                body: vec![],
            },
        };

        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "request");
        assert_eq!(json["tunnel_id"], "my-tunnel");
        assert_eq!(json["payload"]["method"], "GET");
    }

    #[test]
    fn serialize_response() {
        let msg = TunnelMessage::Response {
            request_id: Uuid::nil(),
            payload: ResponsePayload {
                status: 200,
                headers: HashMap::new(),
                body: b"hello world".to_vec(),
            },
        };

        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "response");
        assert_eq!(json["payload"]["status"], 200);
    }

    #[test]
    fn serialize_ping_pong() {
        let ping_json = serde_json::to_value(&TunnelMessage::Ping).unwrap();
        assert_eq!(ping_json["type"], "ping");

        let pong_json = serde_json::to_value(&TunnelMessage::Pong).unwrap();
        assert_eq!(pong_json["type"], "pong");
    }

    #[test]
    fn serialize_request_cancel() {
        let id = Uuid::nil();
        let msg = TunnelMessage::RequestCancel { request_id: id };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "request_cancel");
        assert_eq!(json["request_id"], id.to_string());
    }

    #[test]
    fn roundtrip_request() {
        let msg = TunnelMessage::Request {
            tunnel_id: TunnelId::new("test-abc").unwrap(),
            request_id: Uuid::now_v7(),
            payload: RequestPayload {
                method: "POST".into(),
                path: "/api/data".into(),
                headers: {
                    let mut h = HashMap::new();
                    h.insert("Content-Type".into(), vec!["application/json".into()]);
                    h
                },
                body: b"{\"key\": \"value\"}".to_vec(),
            },
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: TunnelMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            TunnelMessage::Request {
                tunnel_id,
                payload,
                ..
            } => {
                assert_eq!(tunnel_id.as_ref(), "test-abc");
                assert_eq!(payload.method, "POST");
                assert_eq!(payload.body, b"{\"key\": \"value\"}");
            }
            _ => panic!("expected Request variant"),
        }
    }

    #[test]
    fn roundtrip_response_with_body() {
        let body = vec![0u8, 1, 2, 255, 128];
        let msg = TunnelMessage::Response {
            request_id: Uuid::now_v7(),
            payload: ResponsePayload {
                status: 201,
                headers: HashMap::new(),
                body: body.clone(),
            },
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: TunnelMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            TunnelMessage::Response { payload, .. } => {
                assert_eq!(payload.status, 201);
                assert_eq!(payload.body, body);
            }
            _ => panic!("expected Response variant"),
        }
    }

    #[test]
    fn deserialize_unknown_type_fails() {
        let json = r#"{"type": "unknown_msg"}"#;
        let result: Result<TunnelMessage, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
