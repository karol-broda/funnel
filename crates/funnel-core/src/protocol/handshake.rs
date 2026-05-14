use serde::{Deserialize, Serialize};

use crate::protocol::error_codes::AppCode;
use crate::tunnel::id::TunnelId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Handshake {
    pub version: u32,
    pub token: Option<String>,
    pub tunnels: Vec<TunnelSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelSpec {
    pub id: TunnelId,
    #[serde(rename = "type")]
    pub tunnel_type: TunnelType,
    #[serde(default)]
    pub team: Option<String>,
    #[serde(default)]
    pub local_port: Option<u16>,
    #[serde(default)]
    pub routing: Option<RoutingMode>,
    #[serde(default)]
    pub remote_port: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TunnelType {
    Http,
    Stream,
    Dgram,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoutingMode {
    Port,
    Sni,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeResult {
    pub version: u32,
    pub server_id: String,
    pub tunnels: Vec<TunnelResult>,
    pub limits: ServerLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelResult {
    pub id: TunnelId,
    pub status: TunnelStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<AppCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TunnelStatus {
    Ok,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerLimits {
    pub max_streams: u32,
    pub max_request_body: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dgram_mtu: Option<u16>,
}

impl Default for ServerLimits {
    fn default() -> Self {
        Self {
            max_streams: 128,
            max_request_body: 64 * 1024 * 1024,
            dgram_mtu: None,
        }
    }
}

impl TunnelResult {
    pub fn ok(id: TunnelId) -> Self {
        Self {
            id,
            status: TunnelStatus::Ok,
            remote_port: None,
            public_url: None,
            error_code: None,
            error_message: None,
        }
    }

    pub fn ok_with_url(id: TunnelId, url: impl Into<String>) -> Self {
        Self {
            public_url: Some(url.into()),
            ..Self::ok(id)
        }
    }

    pub fn error(
        id: TunnelId,
        code: AppCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id,
            status: TunnelStatus::Error,
            remote_port: None,
            public_url: None,
            error_code: Some(code),
            error_message: Some(message.into()),
        }
    }

    pub fn is_ok(&self) -> bool {
        self.status == TunnelStatus::Ok
    }
}

impl std::fmt::Display for TunnelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http => write!(f, "http"),
            Self::Stream => write!(f, "stream"),
            Self::Dgram => write!(f, "dgram"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::PROTOCOL_VERSION;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn handshake_roundtrip() -> TestResult {
        let handshake = Handshake {
            version: PROTOCOL_VERSION,
            token: Some("test-token".into()),
            tunnels: vec![TunnelSpec {
                id: TunnelId::new("test-tunnel")?,
                tunnel_type: TunnelType::Http,
                team: None,
                local_port: Some(3000),
                routing: None,
                remote_port: None,
            }],
        };

        let encoded = rmp_serde::to_vec_named(&handshake)?;
        let decoded: Handshake = rmp_serde::from_slice(&encoded)?;
        assert_eq!(decoded.version, PROTOCOL_VERSION);
        assert_eq!(decoded.tunnels.len(), 1);
        assert_eq!(decoded.tunnels[0].tunnel_type, TunnelType::Http);
        Ok(())
    }

    #[test]
    fn result_roundtrip() -> TestResult {
        let result = HandshakeResult {
            version: PROTOCOL_VERSION,
            server_id: "abc123".into(),
            tunnels: vec![
                TunnelResult::ok(TunnelId::new("test-tunnel")?),
                TunnelResult::error(
                    TunnelId::new("bad-tunnel")?,
                    AppCode::TunnelIdConflict,
                    "already in use",
                ),
            ],
            limits: ServerLimits::default(),
        };

        let encoded = rmp_serde::to_vec_named(&result)?;
        let decoded: HandshakeResult = rmp_serde::from_slice(&encoded)?;
        assert_eq!(decoded.version, PROTOCOL_VERSION);
        assert_eq!(decoded.tunnels.len(), 2);
        assert!(decoded.tunnels[0].is_ok());
        assert!(!decoded.tunnels[1].is_ok());
        assert_eq!(
            decoded.tunnels[1].error_code,
            Some(AppCode::TunnelIdConflict)
        );
        Ok(())
    }

    #[test]
    fn tunnel_type_serializes_as_lowercase() -> TestResult {
        let spec = TunnelSpec {
            id: TunnelId::new("test")?,
            tunnel_type: TunnelType::Stream,
            team: None,
            local_port: None,
            routing: Some(RoutingMode::Sni),
            remote_port: None,
        };

        let json = serde_json::to_value(&spec)?;
        assert_eq!(json["type"], "stream");
        assert_eq!(json["routing"], "sni");
        Ok(())
    }

}
