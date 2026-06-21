//! Codecora configuration — centralized at `~/.codecora/`.
//!
//! Reads and writes `~/.codecora/config.toml` with layered resolution:
//! defaults → config file → runtime overrides.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Root config structure — maps to `~/.codecora/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodecoraConfig {
    #[serde(default)]
    pub general: GeneralConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub data_dir: Option<String>,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_namespace")]
    pub default_namespace: String,
    #[serde(default = "default_max_results")]
    pub max_results: u32,
}

fn default_theme() -> String {
    "catppuccin-mocha".to_string()
}

fn default_namespace() -> String {
    "default".to_string()
}

fn default_max_results() -> u32 {
    50
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to resolve home directory")]
    NoHomeDir,
    #[error("io error: {0}")]
    Io(String),
    #[error("toml parse error: {0}")]
    Parse(String),
    #[error("toml serialize error: {0}")]
    Serialize(String),
}

/// Resolve the `~/.codecora/` directory path.
///
/// Uses the `dirs` crate for cross-platform home directory resolution.
pub fn codecora_root() -> Result<PathBuf, ConfigError> {
    let home = dirs::home_dir().ok_or(ConfigError::NoHomeDir)?;
    Ok(home.join(".codecora"))
}

/// Resolve the Hub data directory: `~/.codecora/hub/`.
pub fn hub_dir() -> Result<PathBuf, ConfigError> {
    Ok(codecora_root()?.join("hub"))
}

/// Resolve the Hub database path: `~/.codecora/hub/hub.db`.
pub fn hub_db_path() -> Result<PathBuf, ConfigError> {
    Ok(hub_dir()?.join("hub.db"))
}

/// Resolve the config file path: `~/.codecora/config.toml`.
pub fn config_path() -> Result<PathBuf, ConfigError> {
    Ok(codecora_root()?.join("config.toml"))
}

/// Ensure the `~/.codecora/` directory structure exists.
///
/// Creates:
/// - `~/.codecora/`
/// - `~/.codecora/hub/`
/// - `~/.codecora/cache/`
/// - `~/.codecora/logs/`
pub fn ensure_directory_structure() -> Result<PathBuf, ConfigError> {
    let root = codecora_root()?;
    let hub = hub_dir()?;

    for dir in &[&root, &hub, &root.join("cache"), &root.join("logs")] {
        if !dir.exists() {
            fs::create_dir_all(dir).map_err(|e| ConfigError::Io(e.to_string()))?;
        }
    }

    Ok(hub)
}

/// Load config from `~/.codecora/config.toml`.
///
/// Returns default config if file does not exist yet.
pub fn load_config() -> Result<CodecoraConfig, ConfigError> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(CodecoraConfig::default());
    }

    let contents = fs::read_to_string(&path).map_err(|e| ConfigError::Io(e.to_string()))?;
    toml::from_str(&contents).map_err(|e| ConfigError::Parse(e.to_string()))
}

/// Save config to `~/.codecora/config.toml`.
pub fn save_config(config: &CodecoraConfig) -> Result<(), ConfigError> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| ConfigError::Io(e.to_string()))?;
    }

    let contents =
        toml::to_string_pretty(config).map_err(|e| ConfigError::Serialize(e.to_string()))?;

    // Atomic write: write to temp file, then rename
    let tmp = path.with_extension("toml.tmp");
    fs::write(&tmp, contents).map_err(|e| ConfigError::Io(e.to_string()))?;
    fs::rename(&tmp, &path).map_err(|e| ConfigError::Io(e.to_string()))?;

    Ok(())
}

/// Initialize the full Codecora environment:
/// 1. Ensure directory structure exists
/// 2. Load or create config
/// 3. Return the Hub database path
///
/// Called on app startup — replaces the manual directory picker.
pub fn init_environment() -> Result<(PathBuf, CodecoraConfig), ConfigError> {
    ensure_directory_structure()?;

    let config = if config_path()?.exists() {
        load_config()?
    } else {
        let config = CodecoraConfig::default();
        save_config(&config)?;
        config
    };

    let db_path = hub_db_path()?;
    Ok((db_path, config))
}

/// Check if the Codecora environment has been initialized.
pub fn is_initialized() -> bool {
    config_path().map(|p| p.exists()).unwrap_or(false)
}

#[allow(clippy::derivable_impls)]
impl Default for CodecoraConfig {
    fn default() -> Self {
        CodecoraConfig {
            general: GeneralConfig::default(),
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        GeneralConfig {
            data_dir: None,
            theme: default_theme(),
            default_namespace: default_namespace(),
            max_results: default_max_results(),
        }
    }
}

impl GeneralConfig {
    /// Merge another config into self (other takes priority for non-empty values).
    pub fn merge(&mut self, other: &GeneralConfig) {
        if other.data_dir.is_some() {
            self.data_dir = other.data_dir.clone();
        }
        if !other.theme.is_empty() {
            self.theme = other.theme.clone();
        }
        if !other.default_namespace.is_empty() {
            self.default_namespace = other.default_namespace.clone();
        }
        if other.max_results > 0 {
            self.max_results = other.max_results;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CodecoraConfig::default();
        assert_eq!(config.general.theme, "catppuccin-mocha");
        assert_eq!(config.general.default_namespace, "default");
        assert_eq!(config.general.max_results, 50);
    }

    #[test]
    fn test_config_serialization() {
        let config = CodecoraConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("catppuccin-mocha"));
        assert!(toml_str.contains("default"));
    }

    #[test]
    fn test_config_deserialization() {
        let toml_str = r#"[general]
data_dir = "/custom/path"
theme = "light"
default_namespace = "work"
max_results = 100
"#;
        let config: CodecoraConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.general.data_dir, Some("/custom/path".to_string()));
        assert_eq!(config.general.theme, "light");
        assert_eq!(config.general.default_namespace, "work");
        assert_eq!(config.general.max_results, 100);
    }

    #[test]
    fn test_hub_db_path_ends_with_hub_db() {
        let path = hub_db_path().unwrap();
        assert!(path.ends_with("hub.db"));
    }

    #[test]
    fn test_codecora_root_ends_with_codecora() {
        let path = codecora_root().unwrap();
        assert!(path.ends_with(".codecora"));
    }
}
