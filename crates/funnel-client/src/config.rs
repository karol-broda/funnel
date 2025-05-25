use std::collections::HashMap;
use std::path::PathBuf;

use funnel_core::protocol::handshake::AuthScheme;
use serde::{Deserialize, Serialize};

const CONFIG_DIR: &str = "funnel";
const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunnelConfig {
    #[serde(default = "default_context_name")]
    pub current_context: String,
    /// default auth scheme for `funnel http --auth` when no flag is given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_auth_scheme: Option<AuthScheme>,
    #[serde(default)]
    pub contexts: HashMap<String, Context>,
}

impl Default for FunnelConfig {
    fn default() -> Self {
        Self {
            current_context: default_context_name(),
            default_auth_scheme: None,
            contexts: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Context {
    #[serde(default)]
    pub server: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

fn default_context_name() -> String {
    "default".into()
}

pub struct ResolvedContext {
    pub name: String,
    pub server: String,
    pub token: Option<String>,
}

pub fn config_path() -> PathBuf {
    if let Some(dir) = dirs::config_dir() {
        return dir.join(CONFIG_DIR).join(CONFIG_FILE);
    }
    PathBuf::from(CONFIG_FILE)
}

pub fn load() -> anyhow::Result<FunnelConfig> {
    let path = config_path();

    let mut builder = config_rs::Config::builder().set_default("current_context", "default")?;

    if path.exists() {
        builder = builder.add_source(config_rs::File::from(path));
    }

    if let Ok(ctx) = std::env::var("FUNNEL_CONTEXT") {
        builder = builder.set_override("current_context", ctx)?;
    }

    let settings = builder.build()?;
    let config: FunnelConfig = settings.try_deserialize()?;
    Ok(config)
}

pub fn save(config: &FunnelConfig) -> anyhow::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(config)?;
    std::fs::write(&path, content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

/// resolve the active context, applying env var overrides for server and token
pub fn resolve(
    config: &FunnelConfig,
    context_override: Option<&str>,
) -> anyhow::Result<ResolvedContext> {
    let name = context_override.unwrap_or(&config.current_context);

    let ctx = config.contexts.get(name).ok_or_else(|| {
        anyhow::anyhow!(
            "context '{name}' not found, run: funnel context create {name} --server <url>"
        )
    })?;

    let server = std::env::var("FUNNEL_SERVER").unwrap_or_else(|_| ctx.server.clone());

    let token = std::env::var("FUNNEL_TOKEN")
        .ok()
        .or_else(|| ctx.token.clone());

    if server.is_empty() {
        anyhow::bail!("server not configured for context '{name}'");
    }

    Ok(ResolvedContext {
        name: name.to_string(),
        server,
        token,
    })
}

/// resolve context and require a valid token
pub fn resolve_authenticated(
    config: &FunnelConfig,
    context_override: Option<&str>,
) -> anyhow::Result<(ResolvedContext, String)> {
    let resolved = resolve(config, context_override)?;
    let token = resolved.token.clone().ok_or_else(|| {
        anyhow::anyhow!(
            "not logged in for context '{}', run: funnel login",
            resolved.name
        )
    })?;
    Ok((resolved, token))
}

pub fn set_token(context_name: &str, token: &str) -> anyhow::Result<()> {
    let mut config = load()?;
    let ctx = config.contexts.entry(context_name.to_string()).or_default();
    ctx.token = Some(token.to_string());
    save(&config)
}

pub fn clear_token(context_name: &str) -> anyhow::Result<()> {
    let mut config = load()?;
    let ctx = config
        .contexts
        .get_mut(context_name)
        .ok_or_else(|| anyhow::anyhow!("context '{context_name}' not found"))?;
    ctx.token = None;
    save(&config)
}

pub fn set_current_context(name: &str) -> anyhow::Result<()> {
    let mut config = load()?;
    if !config.contexts.contains_key(name) {
        anyhow::bail!("context '{name}' does not exist");
    }
    config.current_context = name.to_string();
    save(&config)
}

pub fn create_context(name: &str, server: &str) -> anyhow::Result<()> {
    let mut config = load()?;
    if config.contexts.contains_key(name) {
        anyhow::bail!("context '{name}' already exists");
    }
    config.contexts.insert(
        name.to_string(),
        Context {
            server: server.to_string(),
            ..Default::default()
        },
    );
    if config.contexts.len() == 1 {
        config.current_context = name.to_string();
    }
    save(&config)
}

pub fn delete_context(name: &str) -> anyhow::Result<()> {
    let mut config = load()?;
    if config.contexts.remove(name).is_none() {
        anyhow::bail!("context '{name}' does not exist");
    }
    if config.current_context == name {
        config.current_context = config
            .contexts
            .keys()
            .next()
            .cloned()
            .unwrap_or_else(default_context_name);
    }
    save(&config)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_no_contexts() {
        let config = FunnelConfig::default();
        assert!(config.contexts.is_empty());
        assert_eq!(config.current_context, "default");
    }

    #[test]
    fn roundtrip_toml() {
        let mut config = FunnelConfig::default();
        config.contexts.insert(
            "default".to_string(),
            Context {
                server: "https://tunnel.example.com".to_string(),
                token: Some("fnl_test123".to_string()),
            },
        );

        let serialized = toml::to_string_pretty(&config).expect("serialize");
        let parsed: FunnelConfig = toml::from_str(&serialized).expect("deserialize");

        let ctx = parsed.contexts.get("default").expect("default context");
        assert_eq!(ctx.server, "https://tunnel.example.com");
        assert_eq!(ctx.token.as_deref(), Some("fnl_test123"));
    }

    #[test]
    fn config_path_is_not_empty() {
        let path = config_path();
        assert!(!path.as_os_str().is_empty());
    }

    #[test]
    fn resolve_missing_context_errors() {
        let config = FunnelConfig::default();
        assert!(resolve(&config, None).is_err());
    }

    #[test]
    fn resolve_uses_current_context() {
        let mut config = FunnelConfig::default();
        config.contexts.insert(
            "default".to_string(),
            Context {
                server: "https://example.com".to_string(),
                token: Some("tok".to_string()),
            },
        );

        let resolved = resolve(&config, None).expect("resolve");
        assert_eq!(resolved.name, "default");
        assert_eq!(resolved.server, "https://example.com");
    }

    #[test]
    fn resolve_override_context() {
        let mut config = FunnelConfig::default();
        config.contexts.insert(
            "staging".to_string(),
            Context {
                server: "https://staging.example.com".to_string(),
                ..Default::default()
            },
        );

        let resolved = resolve(&config, Some("staging")).expect("resolve");
        assert_eq!(resolved.name, "staging");
        assert_eq!(resolved.server, "https://staging.example.com");
    }

    #[test]
    fn resolve_authenticated_requires_token() {
        let mut config = FunnelConfig::default();
        config.contexts.insert(
            "default".to_string(),
            Context {
                server: "https://example.com".to_string(),
                ..Default::default()
            },
        );
        assert!(resolve_authenticated(&config, None).is_err());
    }
}
