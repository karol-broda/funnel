pub mod envelope;
mod types;

pub use envelope::Enveloped;
pub use types::*;

use std::marker::PhantomData;

use http::Method;

pub struct Endpoint<Req, Resp> {
    pub method: Method,
    pub path: &'static str,
    _marker: PhantomData<fn(Req) -> Resp>,
}

pub const HEALTH: Endpoint<(), HealthResponse> = Endpoint {
    method: Method::GET,
    path: "/health",
    _marker: PhantomData,
};

pub const INFO: Endpoint<(), ServerInfo> = Endpoint {
    method: Method::GET,
    path: "/info",
    _marker: PhantomData,
};

pub const ME: Endpoint<(), User> = Endpoint {
    method: Method::GET,
    path: "/me",
    _marker: PhantomData,
};

pub const TUNNELS_LIST: Endpoint<(), Vec<TunnelInfo>> = Endpoint {
    method: Method::GET,
    path: "/tunnels",
    _marker: PhantomData,
};

pub const TUNNELS_GET: Endpoint<(), TunnelInfo> = Endpoint {
    method: Method::GET,
    path: "/tunnels/{id}",
    _marker: PhantomData,
};

pub const TUNNELS_DELETE: Endpoint<(), serde_json::Value> = Endpoint {
    method: Method::DELETE,
    path: "/tunnels/{id}",
    _marker: PhantomData,
};

pub const KEYS_LIST: Endpoint<(), Vec<ApiKeyView>> = Endpoint {
    method: Method::GET,
    path: "/keys",
    _marker: PhantomData,
};

pub const KEYS_CREATE: Endpoint<CreateKeyRequest, CreateKeyResponse> = Endpoint {
    method: Method::POST,
    path: "/keys",
    _marker: PhantomData,
};

pub const KEYS_REVOKE: Endpoint<(), serde_json::Value> = Endpoint {
    method: Method::DELETE,
    path: "/keys/{id}",
    _marker: PhantomData,
};

pub const SESSIONS_LIST: Endpoint<(), Vec<TunnelSession>> = Endpoint {
    method: Method::GET,
    path: "/sessions",
    _marker: PhantomData,
};

pub const ACCOUNTS_LIST: Endpoint<(), Vec<AccountView>> = Endpoint {
    method: Method::GET,
    path: "/accounts",
    _marker: PhantomData,
};

pub const USERS_LIST: Endpoint<(), Vec<User>> = Endpoint {
    method: Method::GET,
    path: "/users",
    _marker: PhantomData,
};

pub const USERS_SET_ROLE: Endpoint<SetUserRoleRequest, User> = Endpoint {
    method: Method::PUT,
    path: "/users/{id}/role",
    _marker: PhantomData,
};

pub const USERS_DEACTIVATE: Endpoint<(), User> = Endpoint {
    method: Method::POST,
    path: "/users/{id}/deactivate",
    _marker: PhantomData,
};

pub const USERS_REACTIVATE: Endpoint<(), User> = Endpoint {
    method: Method::POST,
    path: "/users/{id}/reactivate",
    _marker: PhantomData,
};

pub const TEAMS_LIST: Endpoint<(), Vec<Team>> = Endpoint {
    method: Method::GET,
    path: "/teams",
    _marker: PhantomData,
};

pub const TEAMS_CREATE: Endpoint<CreateTeamRequest, Team> = Endpoint {
    method: Method::POST,
    path: "/teams",
    _marker: PhantomData,
};

pub const TEAMS_DELETE: Endpoint<(), serde_json::Value> = Endpoint {
    method: Method::DELETE,
    path: "/teams/{id}",
    _marker: PhantomData,
};

pub const TEAMS_MEMBERS: Endpoint<(), Vec<TeamMembership>> = Endpoint {
    method: Method::GET,
    path: "/teams/{id}/members",
    _marker: PhantomData,
};

pub const TEAMS_ADD_MEMBER: Endpoint<AddMemberRequest, TeamMembership> = Endpoint {
    method: Method::POST,
    path: "/teams/{id}/members",
    _marker: PhantomData,
};

pub const TEAMS_REMOVE_MEMBER: Endpoint<(), serde_json::Value> = Endpoint {
    method: Method::DELETE,
    path: "/teams/{id}/members/{user_id}",
    _marker: PhantomData,
};

pub const TEAMS_SET_MEMBER_ROLE: Endpoint<SetMemberRoleRequest, TeamMembership> = Endpoint {
    method: Method::PUT,
    path: "/teams/{id}/members/{user_id}/role",
    _marker: PhantomData,
};
