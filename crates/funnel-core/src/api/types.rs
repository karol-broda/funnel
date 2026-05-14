use chrono::{DateTime, Utc};
use funnel_derive::Enveloped;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "db", derive(sqlx::Type))]
#[cfg_attr(feature = "db", sqlx(type_name = "text", rename_all = "lowercase"))]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Member,
}

impl Role {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Member => "member",
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "db", derive(sqlx::Type))]
#[cfg_attr(feature = "db", sqlx(type_name = "text", rename_all = "lowercase"))]
#[serde(rename_all = "lowercase")]
pub enum TeamRole {
    Owner,
    Member,
}

impl TeamRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Member => "member",
        }
    }
}

impl std::fmt::Display for TeamRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum ApiScope {
    Management,
    Tunnels,
}

impl std::fmt::Display for ApiScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Management => f.write_str("management"),
            Self::Tunnels => f.write_str("tunnels"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Enveloped)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub role: Role,
    #[cfg_attr(feature = "openapi", schema(value_type = Object))]
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deactivated_at: Option<DateTime<Utc>>,
}

impl User {
    pub fn is_admin(&self) -> bool {
        self.role == Role::Admin
    }

    pub const fn is_active(&self) -> bool {
        self.deactivated_at.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Enveloped)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Team {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Enveloped)]
#[kind = "member"]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct TeamMembership {
    pub id: Uuid,
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub role: TeamRole,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Enveloped)]
#[kind = "key"]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ApiKeyView {
    pub id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub scopes: Vec<ApiScope>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Enveloped)]
#[kind = "session"]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct TunnelSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tunnel_id: String,
    #[cfg_attr(feature = "openapi", schema(value_type = Option<String>))]
    pub client_ip: Option<ipnetwork::IpNetwork>,
    pub connected_at: DateTime<Utc>,
    pub disconnected_at: Option<DateTime<Utc>>,
    pub bytes_in: i64,
    pub bytes_out: i64,
    pub requests: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Enveloped)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TunnelStatsSnapshot {
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub requests: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Enveloped)]
#[kind = "tunnel"]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TunnelInfo {
    pub id: String,
    pub uptime_secs: f64,
    pub stats: TunnelStatsSnapshot,
    pub owner_id: Uuid,
    pub team_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Enveloped)]
#[kind = "health"]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct HealthResponse {
    pub status: String,
    pub uptime_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Enveloped)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ServerInfo {
    pub version: u32,
    pub quic_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, Enveloped)]
#[kind = "account"]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AccountView {
    pub id: Uuid,
    pub provider: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateKeyRequest {
    pub name: String,
    pub scopes: Option<Vec<ApiScope>>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Enveloped)]
#[kind = "key"]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateKeyResponse {
    pub key: String,
    pub info: ApiKeyView,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SetUserRoleRequest {
    pub role: Role,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CreateTeamRequest {
    pub name: String,
    pub owner_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct AddMemberRequest {
    pub user_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct SetMemberRoleRequest {
    pub role: TeamRole,
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Role --

    #[test]
    fn role_serializes_to_lowercase() {
        assert_eq!(serde_json::to_value(Role::Admin).unwrap(), "admin");
        assert_eq!(serde_json::to_value(Role::Member).unwrap(), "member");
    }

    #[test]
    fn role_deserializes_from_lowercase() {
        assert_eq!(
            serde_json::from_str::<Role>("\"admin\"").unwrap(),
            Role::Admin
        );
        assert_eq!(
            serde_json::from_str::<Role>("\"member\"").unwrap(),
            Role::Member
        );
    }

    #[test]
    fn role_rejects_invalid_values() {
        assert!(serde_json::from_str::<Role>("\"moderator\"").is_err());
        assert!(serde_json::from_str::<Role>("\"\"").is_err());
        assert!(serde_json::from_str::<Role>("\"user\"").is_err());
        assert!(serde_json::from_str::<Role>("42").is_err());
    }

    #[test]
    fn role_rejects_wrong_case() {
        assert!(serde_json::from_str::<Role>("\"Admin\"").is_err());
        assert!(serde_json::from_str::<Role>("\"ADMIN\"").is_err());
        assert!(serde_json::from_str::<Role>("\"Member\"").is_err());
    }

    #[test]
    fn role_display_matches_serialization() {
        assert_eq!(Role::Admin.to_string(), "admin");
        assert_eq!(Role::Member.to_string(), "member");
    }

    #[test]
    fn role_as_str_matches_display() {
        assert_eq!(Role::Admin.as_str(), Role::Admin.to_string());
        assert_eq!(Role::Member.as_str(), Role::Member.to_string());
    }

    #[test]
    fn role_roundtrip_through_json() {
        for role in [Role::Admin, Role::Member] {
            let json = serde_json::to_string(&role).unwrap();
            let parsed: Role = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, role);
        }
    }

    #[test]
    fn role_roundtrip_through_msgpack() {
        for role in [Role::Admin, Role::Member] {
            let bytes = rmp_serde::to_vec_named(&role).unwrap();
            let parsed: Role = rmp_serde::from_slice(&bytes).unwrap();
            assert_eq!(parsed, role);
        }
    }

    // -- TeamRole --

    #[test]
    fn team_role_serializes_to_lowercase() {
        assert_eq!(serde_json::to_value(TeamRole::Owner).unwrap(), "owner");
        assert_eq!(serde_json::to_value(TeamRole::Member).unwrap(), "member");
    }

    #[test]
    fn team_role_deserializes_from_lowercase() {
        assert_eq!(
            serde_json::from_str::<TeamRole>("\"owner\"").unwrap(),
            TeamRole::Owner
        );
        assert_eq!(
            serde_json::from_str::<TeamRole>("\"member\"").unwrap(),
            TeamRole::Member
        );
    }

    #[test]
    fn team_role_rejects_invalid_values() {
        assert!(serde_json::from_str::<TeamRole>("\"admin\"").is_err());
        assert!(serde_json::from_str::<TeamRole>("\"\"").is_err());
        assert!(serde_json::from_str::<TeamRole>("\"manager\"").is_err());
    }

    #[test]
    fn team_role_rejects_wrong_case() {
        assert!(serde_json::from_str::<TeamRole>("\"Owner\"").is_err());
        assert!(serde_json::from_str::<TeamRole>("\"MEMBER\"").is_err());
    }

    #[test]
    fn team_role_display_matches_serialization() {
        assert_eq!(TeamRole::Owner.to_string(), "owner");
        assert_eq!(TeamRole::Member.to_string(), "member");
    }

    #[test]
    fn team_role_roundtrip_through_json() {
        for role in [TeamRole::Owner, TeamRole::Member] {
            let json = serde_json::to_string(&role).unwrap();
            let parsed: TeamRole = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, role);
        }
    }

    // -- ApiScope --

    #[test]
    fn api_scope_serializes_to_lowercase() {
        assert_eq!(
            serde_json::to_value(ApiScope::Management).unwrap(),
            "management"
        );
        assert_eq!(
            serde_json::to_value(ApiScope::Tunnels).unwrap(),
            "tunnels"
        );
    }

    #[test]
    fn api_scope_deserializes_from_lowercase() {
        assert_eq!(
            serde_json::from_str::<ApiScope>("\"management\"").unwrap(),
            ApiScope::Management
        );
        assert_eq!(
            serde_json::from_str::<ApiScope>("\"tunnels\"").unwrap(),
            ApiScope::Tunnels
        );
    }

    #[test]
    fn api_scope_rejects_invalid_values() {
        assert!(serde_json::from_str::<ApiScope>("\"admin\"").is_err());
        assert!(serde_json::from_str::<ApiScope>("\"all\"").is_err());
        assert!(serde_json::from_str::<ApiScope>("\"\"").is_err());
        assert!(serde_json::from_str::<ApiScope>("\"read\"").is_err());
    }

    #[test]
    fn api_scope_rejects_wrong_case() {
        assert!(serde_json::from_str::<ApiScope>("\"Management\"").is_err());
        assert!(serde_json::from_str::<ApiScope>("\"TUNNELS\"").is_err());
    }

    #[test]
    fn api_scope_display_matches_serialization() {
        assert_eq!(ApiScope::Management.to_string(), "management");
        assert_eq!(ApiScope::Tunnels.to_string(), "tunnels");
    }

    #[test]
    fn api_scope_vec_roundtrip() {
        let scopes = vec![ApiScope::Management, ApiScope::Tunnels];
        let json = serde_json::to_string(&scopes).unwrap();
        let parsed: Vec<ApiScope> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, scopes);
    }

    #[test]
    fn api_scope_vec_with_invalid_entry_fails() {
        let json = r#"["management","invalid","tunnels"]"#;
        assert!(serde_json::from_str::<Vec<ApiScope>>(json).is_err());
    }

    // -- request types with enums --

    #[test]
    fn set_user_role_request_deserializes() {
        let json = r#"{"role":"admin"}"#;
        let req: SetUserRoleRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.role, Role::Admin);
    }

    #[test]
    fn set_user_role_request_rejects_invalid_role() {
        let json = r#"{"role":"superadmin"}"#;
        assert!(serde_json::from_str::<SetUserRoleRequest>(json).is_err());
    }

    #[test]
    fn set_member_role_request_deserializes() {
        let json = r#"{"role":"owner"}"#;
        let req: SetMemberRoleRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.role, TeamRole::Owner);
    }

    #[test]
    fn set_member_role_request_rejects_invalid_role() {
        let json = r#"{"role":"admin"}"#;
        assert!(serde_json::from_str::<SetMemberRoleRequest>(json).is_err());
    }

    #[test]
    fn create_key_request_with_valid_scopes() {
        let json = r#"{"name":"test","scopes":["management","tunnels"]}"#;
        let req: CreateKeyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.scopes, Some(vec![ApiScope::Management, ApiScope::Tunnels]));
    }

    #[test]
    fn create_key_request_with_null_scopes() {
        let json = r#"{"name":"test"}"#;
        let req: CreateKeyRequest = serde_json::from_str(json).unwrap();
        assert!(req.scopes.is_none());
    }

    #[test]
    fn create_key_request_rejects_invalid_scope() {
        let json = r#"{"name":"test","scopes":["management","admin"]}"#;
        assert!(serde_json::from_str::<CreateKeyRequest>(json).is_err());
    }
}
