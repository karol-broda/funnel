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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access: Option<AccessControl>,
}

/// which http authentication scheme the server uses to gate the tunnel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthScheme {
    /// `Proxy-Authorization` + `407`; leaves the application's `Authorization`
    /// untouched but browsers do not show a login prompt.
    #[default]
    Proxy,
    /// `Authorization` + `401`; browsers show the native login prompt, but the
    /// header is consumed by the gate and not forwarded to the application.
    Basic,
}

/// access control enforced by the server at the tunnel edge, configured by the
/// authenticated tunnel owner.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessControl {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub basic_auth: Option<String>,
    #[serde(default, skip_serializing_if = "AuthScheme::is_default")]
    pub auth_scheme: AuthScheme,
    /// empty means no ip restriction.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow_ip: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_secs: Option<u64>,
}

impl AuthScheme {
    // signature is dictated by serde's skip_serializing_if, which passes &self
    #[allow(clippy::trivially_copy_pass_by_ref)]
    const fn is_default(&self) -> bool {
        matches!(self, Self::Proxy)
    }
}

impl AccessControl {
    pub const fn is_empty(&self) -> bool {
        self.basic_auth.is_none() && self.allow_ip.is_empty() && self.expires_secs.is_none()
    }
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
    #[serde(default)]
    pub allowed_tunnel_types: Vec<TunnelType>,
}

impl Default for ServerLimits {
    fn default() -> Self {
        Self {
            max_streams: 128,
            max_request_body: 64 * 1024 * 1024,
            dgram_mtu: None,
            allowed_tunnel_types: vec![TunnelType::Http],
        }
    }
}

impl ServerLimits {
    #[must_use]
    pub fn with_tunnel_types(mut self, types: Vec<TunnelType>) -> Self {
        self.allowed_tunnel_types = types;
        self
    }
}

impl TunnelResult {
    pub const fn ok(id: TunnelId) -> Self {
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

    pub fn ok_with_port(id: TunnelId, port: u16) -> Self {
        Self {
            remote_port: Some(port),
            ..Self::ok(id)
        }
    }

    pub fn error(id: TunnelId, code: AppCode, message: impl Into<String>) -> Self {
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
#[allow(clippy::unwrap_used, clippy::expect_used)]
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
                access: None,
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
            access: None,
        };

        let json = serde_json::to_value(&spec)?;
        assert_eq!(json["type"], "stream");
        assert_eq!(json["routing"], "sni");
        Ok(())
    }

    #[test]
    fn access_control_is_empty() {
        assert!(AccessControl::default().is_empty());
        assert!(
            !AccessControl {
                basic_auth: Some("user:pass".into()),
                ..Default::default()
            }
            .is_empty()
        );
    }

    #[test]
    fn tunnel_spec_access_roundtrip() -> TestResult {
        let spec = TunnelSpec {
            id: TunnelId::new("guarded")?,
            tunnel_type: TunnelType::Http,
            team: None,
            local_port: Some(3000),
            routing: None,
            remote_port: None,
            access: Some(AccessControl {
                basic_auth: Some("admin:secret".into()),
                auth_scheme: AuthScheme::Basic,
                allow_ip: vec!["10.0.0.0/8".into()],
                expires_secs: Some(7200),
            }),
        };

        let encoded = rmp_serde::to_vec_named(&spec)?;
        let decoded: TunnelSpec = rmp_serde::from_slice(&encoded)?;
        let access = decoded.access.expect("access present");
        assert_eq!(access.basic_auth.as_deref(), Some("admin:secret"));
        assert_eq!(access.auth_scheme, AuthScheme::Basic);
        assert_eq!(access.allow_ip, vec!["10.0.0.0/8".to_string()]);
        assert_eq!(access.expires_secs, Some(7200));
        Ok(())
    }

    #[test]
    fn auth_scheme_defaults_to_proxy_and_is_omitted() -> TestResult {
        let access = AccessControl {
            basic_auth: Some("admin:secret".into()),
            ..Default::default()
        };
        assert_eq!(access.auth_scheme, AuthScheme::Proxy);

        let json = serde_json::to_value(&access)?;
        assert!(json.get("auth_scheme").is_none());
        Ok(())
    }

    #[test]
    fn tunnel_spec_without_access_omits_field() -> TestResult {
        let spec = TunnelSpec {
            id: TunnelId::new("plain")?,
            tunnel_type: TunnelType::Http,
            team: None,
            local_port: None,
            routing: None,
            remote_port: None,
            access: None,
        };

        let json = serde_json::to_value(&spec)?;
        assert!(json.get("access").is_none());
        Ok(())
    }
}
