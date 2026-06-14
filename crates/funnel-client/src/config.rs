use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const CONFIG_DIR: &str = "funnel";
const CONFIG_FILE: &str = "config.toml";
const PROJECT_CONFIG_FILE: &str = "funnel.toml";

/// Effective client configuration.
///
/// Values can come from the user config at
/// `$XDG_CONFIG_HOME/funnel/config.toml` and from the nearest project
/// `funnel.toml`. Project values override user defaults for commands that read
/// the effective config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunnelConfig {
    /// Name of the context used when `--context` is not provided.
    #[serde(default = "default_context_name")]
    pub current_context: String,
    /// Named server contexts. User config is the intended place for tokens.
    #[serde(default)]
    pub contexts: HashMap<String, Context>,
    /// Local HTTP inspector defaults.
    #[serde(default)]
    pub inspector: InspectorConfig,
}

impl Default for FunnelConfig {
    fn default() -> Self {
        Self {
            current_context: default_context_name(),
            contexts: HashMap::new(),
            inspector: InspectorConfig::default(),
        }
    }
}

/// Defaults for the local HTTP inspector started by `funnel http`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectorConfig {
    /// Start the inspector automatically for HTTP tunnels.
    #[serde(default)]
    pub enabled: bool,
    /// Address the local inspector binds to.
    #[serde(default = "default_inspector_addr")]
    pub addr: String,
}

impl Default for InspectorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            addr: default_inspector_addr(),
        }
    }
}

/// A named tunnel server context.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Context {
    /// Base URL for the funnel server.
    #[serde(default)]
    pub server: String,
    /// Authentication token for the server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

fn default_context_name() -> String {
    "default".into()
}

fn default_inspector_addr() -> String {
    "127.0.0.1:4040".into()
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

pub fn project_config_path() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    find_project_config(&cwd)
}

fn find_project_config(start: &Path) -> Option<PathBuf> {
    for dir in start.ancestors() {
        let candidate = dir.join(PROJECT_CONFIG_FILE);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

pub fn load() -> anyhow::Result<FunnelConfig> {
    load_user()
}

pub fn load_user() -> anyhow::Result<FunnelConfig> {
    load_from_sources(&[config_path()])
}

pub fn load_effective() -> anyhow::Result<FunnelConfig> {
    let mut sources = vec![config_path()];
    if let Some(project_path) = project_config_path() {
        sources.push(project_path);
    }
    load_from_sources(&sources)
}

fn load_from_sources(paths: &[PathBuf]) -> anyhow::Result<FunnelConfig> {
    let mut builder = config_rs::Config::builder().set_default("current_context", "default")?;

    for path in paths {
        if path.exists() {
            builder = builder.add_source(config_rs::File::from(path.clone()));
        }
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

    fn temp_config_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("funnel-test-{name}-{}", uuid::Uuid::now_v7()))
    }

    #[test]
    fn default_config_has_no_contexts() {
        let config = FunnelConfig::default();
        assert!(config.contexts.is_empty());
        assert_eq!(config.current_context, "default");
        assert!(!config.inspector.enabled);
        assert_eq!(config.inspector.addr, "127.0.0.1:4040");
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
        assert!(!parsed.inspector.enabled);
    }

    #[test]
    fn inspector_config_roundtrip() {
        let config = FunnelConfig {
            inspector: InspectorConfig {
                enabled: true,
                addr: "127.0.0.1:5050".to_string(),
            },
            ..Default::default()
        };

        let serialized = toml::to_string_pretty(&config).expect("serialize");
        let parsed: FunnelConfig = toml::from_str(&serialized).expect("deserialize");

        assert!(parsed.inspector.enabled);
        assert_eq!(parsed.inspector.addr, "127.0.0.1:5050");
    }

    #[test]
    fn config_path_is_not_empty() {
        let path = config_path();
        assert!(!path.as_os_str().is_empty());
    }

    #[test]
    fn finds_project_config_in_parent_directory() {
        let root = temp_config_dir("project-discovery");
        let nested = root.join("apps").join("web");
        std::fs::create_dir_all(&nested).expect("create nested dir");
        std::fs::write(root.join(PROJECT_CONFIG_FILE), "").expect("write project config");

        let found = find_project_config(&nested).expect("project config");
        assert_eq!(found, root.join(PROJECT_CONFIG_FILE));

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn project_config_overrides_user_defaults() {
        let root = temp_config_dir("merge");
        std::fs::create_dir_all(&root).expect("create temp dir");
        let user_path = root.join("user.toml");
        let project_path = root.join(PROJECT_CONFIG_FILE);

        std::fs::write(
            &user_path,
            r#"
current_context = "default"

[inspector]
enabled = false
addr = "127.0.0.1:4040"

[contexts.default]
server = "https://user.example.com"
token = "user-token"
"#,
        )
        .expect("write user config");

        std::fs::write(
            &project_path,
            r#"
[inspector]
enabled = true

[contexts.default]
server = "https://project.example.com"
"#,
        )
        .expect("write project config");

        let config = load_from_sources(&[user_path, project_path]).expect("load config");
        assert!(config.inspector.enabled);
        assert_eq!(config.inspector.addr, "127.0.0.1:4040");
        let ctx = config.contexts.get("default").expect("default context");
        assert_eq!(ctx.server, "https://project.example.com");
        assert_eq!(ctx.token.as_deref(), Some("user-token"));

        std::fs::remove_dir_all(root).expect("cleanup");
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
