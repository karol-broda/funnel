use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};

pub struct TagSeo {
    pub tag: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub keywords: &'static [&'static str],
}

impl TagSeo {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "title": self.title,
            "description": self.description,
            "keywords": self.keywords,
        })
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Funnel API",
        description = "Funnel tunnel server REST API",
        version = env!("CARGO_PKG_VERSION"),
        license(name = "MIT"),
    ),
    servers(
        (url = "/api/v1", description = "API v1"),
    ),
    paths(
        crate::api::health::handler,
        crate::api::info::handler,
        crate::api::tunnels::list,
        crate::api::tunnels::get_tunnel,
        crate::api::tunnels::delete,
        crate::api::keys::list,
        crate::api::keys::create,
        crate::api::keys::revoke,
        crate::api::me::handler,
        crate::api::accounts::list,
        crate::api::sessions::list,
        crate::api::users::list,
        crate::api::users::set_role,
        crate::api::users::deactivate,
        crate::api::users::reactivate,
        crate::api::teams::list,
        crate::api::teams::create,
        crate::api::teams::delete,
        crate::api::teams::list_members,
        crate::api::teams::add_member,
        crate::api::teams::remove_member,
        crate::api::teams::set_member_role,
    ),
    components(
        schemas(
            funnel_core::api::HealthResponse,
            funnel_core::api::ServerInfo,
            funnel_core::api::TunnelInfo,
            funnel_core::api::TunnelStatsSnapshot,
            funnel_core::api::ApiKeyView,
            funnel_core::api::CreateKeyRequest,
            funnel_core::api::CreateKeyResponse,
            funnel_core::api::User,
            funnel_core::api::Team,
            funnel_core::api::TeamMembership,
            funnel_core::api::TunnelSession,
            funnel_core::api::AccountView,
            funnel_core::api::envelope::ErrorData,
            funnel_core::api::SetUserRoleRequest,
            funnel_core::api::CreateTeamRequest,
            funnel_core::api::AddMemberRequest,
            funnel_core::api::SetMemberRoleRequest,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Server", description = "Server health and info"),
        (name = "Tunnels", description = "Tunnel management"),
        (name = "API Keys", description = "API key management"),
        (name = "Profile", description = "Current user profile and sessions"),
        (name = "Users", description = "User management (admin only)"),
        (name = "Teams", description = "Team management"),
    ),
)]
struct ApiDoc;

struct SecurityAddon;

const TAG_SEO_ENTRIES: &[&TagSeo] = &[
    &crate::api::health::TAG_SEO,
    &crate::api::tunnels::TAG_SEO,
    &crate::api::keys::TAG_SEO,
    &crate::api::me::TAG_SEO,
    &crate::api::users::TAG_SEO,
    &crate::api::teams::TAG_SEO,
];

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );

        if let Some(tags) = &mut openapi.tags {
            for tag in tags {
                let extensions = tag.extensions.get_or_insert_with(Default::default);

                // without this, fumadocs idToTitle() mangles "API Keys" into "A P I Keys"
                extensions.insert("x-displayName".to_string(), serde_json::json!(tag.name));

                if let Some(seo) = TAG_SEO_ENTRIES.iter().find(|s| s.tag == tag.name) {
                    extensions.insert("x-seo".to_string(), seo.to_json());
                }
            }
        }
    }
}

pub fn spec() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

pub async fn json_handler() -> axum::Json<utoipa::openapi::OpenApi> {
    axum::Json(spec())
}
