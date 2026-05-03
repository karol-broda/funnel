use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const CONFIG_DIR: &str = "funnel";
const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub inlets: HashMap<String, Inlet>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Inlet {
    #[serde(default)]
    pub server: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

pub fn config_path() -> PathBuf {
    if let Some(dir) = dirs::config_dir() {
        return dir.join(CONFIG_DIR).join(CONFIG_FILE);
    }
    PathBuf::from(CONFIG_FILE)
}

pub fn load() -> anyhow::Result<Config> {
    let path = config_path();
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = std::fs::read_to_string(&path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

pub fn save(config: &Config) -> anyhow::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(config)?;
    std::fs::write(&path, content)?;

    // restrict permissions on unix (file contains tokens)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

pub fn get_inlet<'a>(config: &'a Config, name: &str) -> Option<&'a Inlet> {
    config.inlets.get(name)
}

pub fn set_token(inlet_name: &str, token: &str) -> anyhow::Result<()> {
    let mut config = load()?;
    let inlet = config.inlets.entry(inlet_name.to_string()).or_default();
    inlet.token = Some(token.to_string());
    save(&config)
}

pub fn set_server(inlet_name: &str, server: &str) -> anyhow::Result<()> {
    let mut config = load()?;
    let inlet = config.inlets.entry(inlet_name.to_string()).or_default();
    inlet.server = server.to_string();
    save(&config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_no_inlets() {
        let config = Config::default();
        assert!(config.inlets.is_empty());
    }

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn roundtrip_toml() -> TestResult {
        let mut config = Config::default();
        config.inlets.insert(
            "default".to_string(),
            Inlet {
                server: "https://tunnel.example.com".to_string(),
                domain: None,
                token: Some("sk_test123".to_string()),
            },
        );

        let serialized = toml::to_string_pretty(&config)?;
        let parsed: Config = toml::from_str(&serialized)?;

        let inlet = parsed
            .inlets
            .get("default")
            .ok_or("expected default inlet")?;
        assert_eq!(inlet.server, "https://tunnel.example.com");
        assert_eq!(inlet.token.as_deref(), Some("sk_test123"));
        assert!(inlet.domain.is_none());
        Ok(())
    }

    #[test]
    fn get_inlet_returns_none_for_missing() {
        let config = Config::default();
        assert!(get_inlet(&config, "default").is_none());
    }

    #[test]
    fn config_path_is_not_empty() {
        let path = config_path();
        assert!(!path.as_os_str().is_empty());
    }
}
