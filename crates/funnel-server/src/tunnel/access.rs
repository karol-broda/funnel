use std::net::IpAddr;

use axum::http::HeaderMap;
use base64::Engine;
use ipnetwork::IpNetwork;
use tokio::time::{Duration, Instant};

use funnel_core::protocol::handshake::AccessControl;

/// reason an incoming request was rejected at the tunnel edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessDenied {
    Expired,
    IpForbidden,
    ProxyAuthRequired,
}

/// access control resolved from a tunnel spec and enforced per request.
#[derive(Debug, Default, Clone)]
pub struct AccessPolicy {
    /// precomputed `Basic <base64>` value matched against `Proxy-Authorization`.
    expected_proxy_authorization: Option<String>,
    allow_networks: Vec<IpNetwork>,
    expires_at: Option<Instant>,
}

impl AccessPolicy {
    /// `connected_at` anchors the expiry clock. errors when an allowlist entry
    /// is not valid cidr notation.
    pub fn from_spec(
        access: Option<&AccessControl>,
        connected_at: Instant,
    ) -> Result<Self, String> {
        let Some(access) = access else {
            return Ok(Self::default());
        };

        let expected_proxy_authorization = access.basic_auth.as_ref().map(|creds| {
            let encoded = base64::engine::general_purpose::STANDARD.encode(creds);
            format!("Basic {encoded}")
        });

        let mut allow_networks = Vec::with_capacity(access.allow_ip.len());
        for entry in &access.allow_ip {
            let network = entry
                .parse::<IpNetwork>()
                .map_err(|_| format!("invalid cidr in allow-ip: {entry}"))?;
            allow_networks.push(network);
        }

        let expires_at = access
            .expires_secs
            .map(|secs| connected_at + Duration::from_secs(secs));

        Ok(Self {
            expected_proxy_authorization,
            allow_networks,
            expires_at,
        })
    }

    pub const fn expires_at(&self) -> Option<Instant> {
        self.expires_at
    }

    /// check an incoming request against the policy. checks run cheapest first:
    /// expiry, then peer address, then credentials.
    pub fn check(
        &self,
        headers: &HeaderMap,
        peer_ip: IpAddr,
        now: Instant,
    ) -> Result<(), AccessDenied> {
        if self.expires_at.is_some_and(|expires_at| now >= expires_at) {
            return Err(AccessDenied::Expired);
        }

        if !self.allow_networks.is_empty()
            && !self.allow_networks.iter().any(|net| net.contains(peer_ip))
        {
            return Err(AccessDenied::IpForbidden);
        }

        if let Some(expected) = &self.expected_proxy_authorization {
            let provided = headers
                .get(axum::http::header::PROXY_AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            if !constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
                return Err(AccessDenied::ProxyAuthRequired);
            }
        }

        Ok(())
    }
}

/// compare two byte slices without short circuiting on the first mismatch.
/// length is not secret here, so an early length check is acceptable.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn proxy_auth_header(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::PROXY_AUTHORIZATION,
            value.parse().expect("valid header value"),
        );
        headers
    }

    fn basic(creds: &str) -> String {
        let encoded = base64::engine::general_purpose::STANDARD.encode(creds);
        format!("Basic {encoded}")
    }

    #[test]
    fn empty_policy_allows_everything() {
        let policy = AccessPolicy::default();
        let ip = "203.0.113.1".parse().unwrap();
        assert!(policy.check(&HeaderMap::new(), ip, Instant::now()).is_ok());
    }

    #[test]
    fn basic_auth_accepts_matching_credentials() {
        let access = AccessControl {
            basic_auth: Some("admin:secret".into()),
            ..Default::default()
        };
        let policy = AccessPolicy::from_spec(Some(&access), Instant::now()).unwrap();
        let ip = "203.0.113.1".parse().unwrap();
        let headers = proxy_auth_header(&basic("admin:secret"));
        assert!(policy.check(&headers, ip, Instant::now()).is_ok());
    }

    #[test]
    fn basic_auth_rejects_missing_or_wrong_credentials() {
        let access = AccessControl {
            basic_auth: Some("admin:secret".into()),
            ..Default::default()
        };
        let policy = AccessPolicy::from_spec(Some(&access), Instant::now()).unwrap();
        let ip = "203.0.113.1".parse().unwrap();

        assert_eq!(
            policy.check(&HeaderMap::new(), ip, Instant::now()),
            Err(AccessDenied::ProxyAuthRequired)
        );
        let headers = proxy_auth_header(&basic("admin:wrong"));
        assert_eq!(
            policy.check(&headers, ip, Instant::now()),
            Err(AccessDenied::ProxyAuthRequired)
        );
    }

    #[test]
    fn basic_auth_ignores_the_application_authorization_header() {
        let access = AccessControl {
            basic_auth: Some("admin:secret".into()),
            ..Default::default()
        };
        let policy = AccessPolicy::from_spec(Some(&access), Instant::now()).unwrap();
        let ip = "203.0.113.1".parse().unwrap();

        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            basic("admin:secret").parse().unwrap(),
        );
        assert_eq!(
            policy.check(&headers, ip, Instant::now()),
            Err(AccessDenied::ProxyAuthRequired)
        );
    }

    #[test]
    fn ip_allowlist_permits_only_listed_ranges() {
        let access = AccessControl {
            allow_ip: vec!["10.0.0.0/8".into()],
            ..Default::default()
        };
        let policy = AccessPolicy::from_spec(Some(&access), Instant::now()).unwrap();

        let allowed = "10.1.2.3".parse().unwrap();
        assert!(
            policy
                .check(&HeaderMap::new(), allowed, Instant::now())
                .is_ok()
        );

        let blocked = "203.0.113.1".parse().unwrap();
        assert_eq!(
            policy.check(&HeaderMap::new(), blocked, Instant::now()),
            Err(AccessDenied::IpForbidden)
        );
    }

    #[test]
    fn ip_allowlist_matches_any_of_multiple_ranges() {
        let access = AccessControl {
            allow_ip: vec!["10.0.0.0/8".into(), "192.168.0.0/16".into()],
            ..Default::default()
        };
        let policy = AccessPolicy::from_spec(Some(&access), Instant::now()).unwrap();

        for allowed in ["10.1.2.3", "192.168.5.6"] {
            let ip = allowed.parse().unwrap();
            assert!(policy.check(&HeaderMap::new(), ip, Instant::now()).is_ok());
        }

        let blocked = "172.16.0.1".parse().unwrap();
        assert_eq!(
            policy.check(&HeaderMap::new(), blocked, Instant::now()),
            Err(AccessDenied::IpForbidden)
        );
    }

    #[test]
    fn invalid_cidr_is_rejected() {
        let access = AccessControl {
            allow_ip: vec!["not-a-cidr".into()],
            ..Default::default()
        };
        assert!(AccessPolicy::from_spec(Some(&access), Instant::now()).is_err());
    }

    #[test]
    fn expiry_blocks_requests_after_deadline() {
        let connected_at = Instant::now();
        let access = AccessControl {
            expires_secs: Some(60),
            ..Default::default()
        };
        let policy = AccessPolicy::from_spec(Some(&access), connected_at).unwrap();
        let ip = "203.0.113.1".parse().unwrap();

        assert!(policy.check(&HeaderMap::new(), ip, connected_at).is_ok());
        let after = connected_at + Duration::from_secs(61);
        assert_eq!(
            policy.check(&HeaderMap::new(), ip, after),
            Err(AccessDenied::Expired)
        );
    }

    #[test]
    fn expiry_is_checked_before_other_rules() {
        let connected_at = Instant::now();
        let access = AccessControl {
            basic_auth: Some("admin:secret".into()),
            expires_secs: Some(60),
            ..Default::default()
        };
        let policy = AccessPolicy::from_spec(Some(&access), connected_at).unwrap();
        let ip = "203.0.113.1".parse().unwrap();
        let after = connected_at + Duration::from_secs(61);
        assert_eq!(
            policy.check(&HeaderMap::new(), ip, after),
            Err(AccessDenied::Expired)
        );
    }

    #[test]
    fn constant_time_eq_matches_std_eq() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
    }
}
