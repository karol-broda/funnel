use serde::{Deserialize, Serialize};

/// quic connection level error codes, used in `Connection::close()`.
/// fatal, all tunnels on the connection are torn down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ConnectionCode {
    NoError = 0x00,
    ProtocolError = 0x01,
    AuthFailed = 0x02,
    VersionMismatch = 0x03,
    InternalError = 0x04,
    ShuttingDown = 0x05,
}

impl ConnectionCode {
    pub const fn as_u32(self) -> u32 {
        self as u32
    }
}

/// quic stream level error codes, used in `SendStream::reset()`.
/// affects a single stream without tearing down the connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum StreamCode {
    NoError = 0x00,
    TunnelGone = 0x01,
    RateLimited = 0x02,
    AccessDenied = 0x03,
    Timeout = 0x04,
    LocalUnreachable = 0x05,
    BodyTooLarge = 0x06,
}

impl StreamCode {
    pub const fn as_u32(self) -> u32 {
        self as u32
    }
}

/// application level error codes shared across the wire protocol,
/// rest api, and cli `--json` output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppCode {
    TunnelIdConflict,
    TunnelIdInvalid,
    UnsupportedTunnelType,
    PortUnavailable,
    AuthRequired,
    AuthInvalid,
    ScopeInsufficient,
    UserDeactivated,
    TeamNotFound,
    TeamMembershipRequired,
    TunnelLimitExceeded,
    RateLimitExceeded,
    BadRequest,
    NotFound,
    InternalError,
}

impl AppCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TunnelIdConflict => "tunnel_id_conflict",
            Self::TunnelIdInvalid => "tunnel_id_invalid",
            Self::UnsupportedTunnelType => "unsupported_tunnel_type",
            Self::PortUnavailable => "port_unavailable",
            Self::AuthRequired => "auth_required",
            Self::AuthInvalid => "auth_invalid",
            Self::ScopeInsufficient => "scope_insufficient",
            Self::UserDeactivated => "user_deactivated",
            Self::TeamNotFound => "team_not_found",
            Self::TeamMembershipRequired => "team_membership_required",
            Self::TunnelLimitExceeded => "tunnel_limit_exceeded",
            Self::RateLimitExceeded => "rate_limit_exceeded",
            Self::BadRequest => "bad_request",
            Self::NotFound => "not_found",
            Self::InternalError => "internal_error",
        }
    }
}

impl std::fmt::Display for AppCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_codes_have_expected_values() {
        assert_eq!(ConnectionCode::NoError.as_u32(), 0x00);
        assert_eq!(ConnectionCode::ProtocolError.as_u32(), 0x01);
        assert_eq!(ConnectionCode::AuthFailed.as_u32(), 0x02);
        assert_eq!(ConnectionCode::VersionMismatch.as_u32(), 0x03);
        assert_eq!(ConnectionCode::InternalError.as_u32(), 0x04);
        assert_eq!(ConnectionCode::ShuttingDown.as_u32(), 0x05);
    }

    #[test]
    fn stream_codes_have_expected_values() {
        assert_eq!(StreamCode::NoError.as_u32(), 0x00);
        assert_eq!(StreamCode::TunnelGone.as_u32(), 0x01);
        assert_eq!(StreamCode::RateLimited.as_u32(), 0x02);
        assert_eq!(StreamCode::AccessDenied.as_u32(), 0x03);
        assert_eq!(StreamCode::Timeout.as_u32(), 0x04);
        assert_eq!(StreamCode::LocalUnreachable.as_u32(), 0x05);
        assert_eq!(StreamCode::BodyTooLarge.as_u32(), 0x06);
    }

    #[test]
    fn connection_and_stream_codes_dont_overlap_except_no_error() {
        // both have NoError = 0, which is intentional
        assert_eq!(ConnectionCode::NoError.as_u32(), StreamCode::NoError.as_u32());
        // but their error codes occupy different semantic spaces
        // (this test just documents the intentional overlap)
    }

    #[test]
    fn app_code_as_str_is_snake_case() {
        let all_codes = [
            AppCode::TunnelIdConflict,
            AppCode::TunnelIdInvalid,
            AppCode::UnsupportedTunnelType,
            AppCode::PortUnavailable,
            AppCode::AuthRequired,
            AppCode::AuthInvalid,
            AppCode::ScopeInsufficient,
            AppCode::UserDeactivated,
            AppCode::TeamNotFound,
            AppCode::TeamMembershipRequired,
            AppCode::TunnelLimitExceeded,
            AppCode::RateLimitExceeded,
            AppCode::BadRequest,
            AppCode::NotFound,
            AppCode::InternalError,
        ];
        for code in all_codes {
            let s = code.as_str();
            assert!(!s.is_empty(), "{code:?} has empty as_str()");
            assert!(
                s.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{code:?} as_str() is not snake_case: {s}"
            );
        }
    }

    #[test]
    fn app_code_display_matches_as_str() {
        let codes = [
            AppCode::TunnelIdConflict,
            AppCode::AuthRequired,
            AppCode::NotFound,
            AppCode::InternalError,
            AppCode::BadRequest,
        ];
        for code in codes {
            assert_eq!(code.to_string(), code.as_str());
        }
    }

    #[test]
    fn app_code_serde_roundtrip() {
        let codes = [
            AppCode::TunnelIdConflict,
            AppCode::ScopeInsufficient,
            AppCode::BadRequest,
            AppCode::NotFound,
        ];
        for code in codes {
            let json = serde_json::to_string(&code).unwrap();
            let parsed: AppCode = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, code, "roundtrip failed for {code:?}");
        }
    }

    #[test]
    fn app_code_serializes_as_snake_case_string() {
        assert_eq!(
            serde_json::to_value(AppCode::TunnelIdConflict).unwrap(),
            "tunnel_id_conflict"
        );
        assert_eq!(
            serde_json::to_value(AppCode::AuthRequired).unwrap(),
            "auth_required"
        );
        assert_eq!(
            serde_json::to_value(AppCode::BadRequest).unwrap(),
            "bad_request"
        );
    }

    #[test]
    fn app_code_rejects_invalid_strings() {
        assert!(serde_json::from_str::<AppCode>("\"unknown_error\"").is_err());
        assert!(serde_json::from_str::<AppCode>("\"\"").is_err());
        assert!(serde_json::from_str::<AppCode>("\"TUNNEL_ID_CONFLICT\"").is_err());
    }

    #[test]
    fn app_code_serde_matches_as_str() {
        // the json serialization (via serde rename_all) must match as_str()
        // since as_str() is used on the wire and in error responses
        let codes = [
            AppCode::TunnelIdConflict,
            AppCode::TunnelIdInvalid,
            AppCode::UnsupportedTunnelType,
            AppCode::PortUnavailable,
            AppCode::AuthRequired,
            AppCode::AuthInvalid,
            AppCode::ScopeInsufficient,
            AppCode::UserDeactivated,
            AppCode::TeamNotFound,
            AppCode::TeamMembershipRequired,
            AppCode::TunnelLimitExceeded,
            AppCode::RateLimitExceeded,
            AppCode::BadRequest,
            AppCode::NotFound,
            AppCode::InternalError,
        ];
        for code in codes {
            let json_str = serde_json::to_value(code).unwrap();
            assert_eq!(
                json_str.as_str().unwrap(),
                code.as_str(),
                "serde and as_str() disagree for {code:?}"
            );
        }
    }
}
