//! Codecora configuration — centralized at `~/.codecora/`.
//!
//! Reads and writes `~/.codecora/config.toml` with layered resolution:
//! defaults → config file → runtime overrides.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Default Uteke serve URL when config is missing or unreadable.
const DEFAULT_SERVE_URL: &str = "http://127.0.0.1:8767";

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

/// Resolve the CorIn data directory: `~/.codecora/corin/`.
pub fn corin_dir() -> Result<PathBuf, ConfigError> {
    Ok(codecora_root()?.join("corin"))
}

/// Resolve the CorIn database path: `~/.codecora/corin/corin.db`.
pub fn corin_db_path() -> Result<PathBuf, ConfigError> {
    Ok(corin_dir()?.join("corin.db"))
}

/// Detect the Uteke serve URL by reading Uteke config files.
///
/// Tries both config files in order:
/// 1. `~/.uteke/uteke.toml` (main config, section `[server]`)
/// 2. `~/.uteke/config.toml` (legacy/fallback)
///
/// Reads `[server].host` and `[server].port`, falling back to
/// `DEFAULT_SERVE_URL` (`http://127.0.0.1:8767`) when config is missing
/// or the `[server]` section is absent (all keys commented out).
pub fn detect_uteke_serve_url() -> String {
    let Some(home) = dirs::home_dir() else {
        return DEFAULT_SERVE_URL.to_string();
    };

    // Try both config files: uteke.toml first, then config.toml.
    for name in &["uteke.toml", "config.toml"] {
        let path = home.join(format!(".uteke/{name}"));
        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(parsed) = contents.parse::<toml::Value>() else {
            continue;
        };

        // Uteke uses [server] section with `host` and `port` keys.
        // Also check [serve] as a fallback for alternative spellings.
        let section = parsed.get("server").or_else(|| parsed.get("serve"));
        let Some(section) = section else {
            continue;
        };

        let port = section.get("port").and_then(|p| {
            p.as_integer()
                .map(|i| i.to_string())
                .or_else(|| p.as_str().map(String::from))
        });

        // Port found — build the URL.
        if let Some(port) = port {
            let host = section
                .get("host")
                .or_else(|| section.get("bind"))
                .and_then(|h| h.as_str())
                .unwrap_or("127.0.0.1");
            return format!("http://{host}:{port}");
        }

        // Section exists but port not set — section is commented out.
        // Continue to next config file or fall through to default.
    }

    DEFAULT_SERVE_URL.to_string()
}

/// Resolve the uteke server configuration with full priority chain.
///
/// Priority:
/// 1. Primary connection in DB (connections table, is_primary=1)
/// 2. `UTEKE_SERVER_URL` environment variable
/// 3. `~/.uteke/uteke.toml` / `config.toml` [server] section
/// 4. `DEFAULT_SERVE_URL` fallback
///
/// Returns (url, auth_token).
pub fn resolve_uteke_server(conn: Option<&rusqlite::Connection>) -> (String, Option<String>) {
    // 1. DB primary connection (highest priority).
    if let Some(conn) = conn {
        if let Ok(Some(row)) = crate::connections::store::get_primary(
            conn,
            crate::connections::ProductType::Uteke,
        ) {
            return (row.url, row.auth_token);
        }
    }

    // 2. Environment variable.
    if let Ok(url) = std::env::var("UTEKE_SERVER_URL") {
        let token = std::env::var("UTEKE_AUTH_TOKEN").ok();
        return (url, token);
    }

    // 3. TOML config (legacy).
    let url = detect_uteke_serve_url();
    (url, None)
}

/// Whether a URL points to a remote (non-local) server.
pub fn is_remote_url(url: &str) -> bool {
    let host = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .and_then(|rest| rest.split_once(':').or_else(|| rest.split_once('/')))
        .map(|(h, _)| h.to_string());

    match host {
        Some(h) => h != "127.0.0.1" && h != "localhost" && h != "0.0.0.0",
        None => false,
    }
}

/// Resolve the Uteke symlink directory: `~/.codecora/uteke/`.
///
/// This is a symlink to the actual Uteke data directory (`~/.uteke/` by default).
/// Hub reads Uteke data through this symlink for Phase 2 integration.
pub fn uteke_symlink_path() -> Result<PathBuf, ConfigError> {
    Ok(codecora_root()?.join("uteke"))
}

/// Resolve the ONNX model directory.
///
/// Priority:
/// 1. `~/.uteke/models/embeddinggemma-q4/` — reuse Uteke's model (no duplicate download)
/// 2. `~/.codecora/uteke/models/embeddinggemma-q4/` — CorIn standalone copy
///
/// Returns the first path that contains the model files.
pub fn resolve_model_dir() -> Option<PathBuf> {
    let home = dirs::home_dir()?;

    // 1. Try Uteke's model directory first
    let uteke_models = home.join(".uteke/models/embeddinggemma-q4");
    if uteke_models.join("onnx/model_q4.onnx").exists() {
        return Some(uteke_models);
    }

    // 2. Try CorIn's standalone model directory
    let corin_models = codecora_root().ok()?.join("uteke/models/embeddinggemma-q4");
    if corin_models.join("onnx/model_q4.onnx").exists() {
        return Some(corin_models);
    }

    None
}

/// Get the model directory path for CorIn standalone use.
/// This is where models are downloaded if Uteke is not installed.
pub fn corin_model_dir() -> Result<PathBuf, ConfigError> {
    Ok(codecora_root()?.join("uteke/models/embeddinggemma-q4"))
}

/// Detect if Uteke is installed on this machine.
///
/// Checks common locations:
/// - `~/.uteke/uteke.db`
/// - Custom path from `~/.uteke/config.toml` `[store].path`
///
/// Returns the path to the Uteke data directory if found.
pub fn detect_uteke() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let default_uteke = home.join(".uteke");

    // Check default location
    if default_uteke.join("uteke.db").exists() {
        return Some(default_uteke);
    }

    // Check config.toml for custom path
    let config = default_uteke.join("config.toml");
    if config.exists()
        && let Ok(contents) = fs::read_to_string(&config)
        && let Ok(parsed) = contents.parse::<toml::Value>()
        && let Some(path) = parsed
            .get("store")
            .and_then(|s| s.get("path"))
            .and_then(|p| p.as_str())
    {
        let expanded = expand_tilde(path);
        if expanded.join("uteke.db").exists() {
            return Some(expanded);
        }
    }

    None
}

/// Create a symlink from `~/.codecora/uteke/` → actual Uteke data dir.
///
/// If Uteke is not installed, this is a no-op.
/// If symlink already exists and points correctly, this is a no-op.
/// If symlink exists but points wrong, it is recreated.
///
/// Returns the resolved Uteke path if linked successfully.
pub fn link_uteke() -> Result<Option<PathBuf>, ConfigError> {
    let Some(uteke_path) = detect_uteke() else {
        return Ok(None);
    };

    let symlink = uteke_symlink_path()?;

    // Check if symlink already correct
    if symlink.exists() {
        if let Ok(target) = fs::read_link(&symlink)
            && target == uteke_path
        {
            return Ok(Some(uteke_path));
        }
        // Wrong target — remove and recreate.
        // Use remove_dir for directory symlinks on Windows,
        // remove_file for file symlinks on Unix.
        let _ = fs::remove_file(&symlink).or_else(|_| fs::remove_dir(&symlink));
    }

    // Create symlink
    #[cfg(unix)]
    std::os::unix::fs::symlink(&uteke_path, &symlink)
        .map_err(|e| ConfigError::Io(e.to_string()))?;

    #[cfg(windows)]
    {
        // On Windows, create a directory symlink (requires admin or developer mode)
        std::os::windows::fs::symlink_dir(&uteke_path, &symlink)
            .map_err(|e| ConfigError::Io(e.to_string()))?;
    }

    Ok(Some(uteke_path))
}

/// Expand `~` in a path string to the home directory.
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(stripped);
    }
    PathBuf::from(path)
}

/// Resolve the config file path: `~/.codecora/config.toml`.
pub fn config_path() -> Result<PathBuf, ConfigError> {
    Ok(codecora_root()?.join("config.toml"))
}

/// Ensure the `~/.codecora/` directory structure exists.
///
/// Creates:
/// - `~/.codecora/`
/// - `~/.codecora/corin/`
/// - `~/.codecora/cache/`
/// - `~/.codecora/logs/`
///
/// Also runs legacy migration: if `~/.codecora/hub/hub.db` exists, it is
/// moved to `~/.codecora/corin/corin.db` and the old `hub/` directory is
/// removed.
pub fn ensure_directory_structure() -> Result<PathBuf, ConfigError> {
    let root = codecora_root()?;
    let corin = corin_dir()?;

    for dir in &[&root, &corin, &root.join("cache"), &root.join("logs")] {
        if !dir.exists() {
            fs::create_dir_all(dir).map_err(|e| ConfigError::Io(e.to_string()))?;
        }
    }

    // Legacy migration: ~/.codecora/hub/hub.db → ~/.codecora/corin/corin.db
    migrate_legacy_hub_db(&root, &corin)?;

    Ok(corin)
}

/// Migrate the old Hub database to the new CorIn path.
///
/// - Source: `~/.codecora/hub/hub.db`
/// - Target: `~/.codecora/corin/corin.db`
///
/// If the source exists and the target does not, the file is moved.
/// If both exist, the legacy file is left in place (user must decide).
/// After a successful move, the now-empty `hub/` directory is removed.
fn migrate_legacy_hub_db(root: &Path, corin_dir: &Path) -> Result<(), ConfigError> {
    let legacy_hub_dir = root.join("hub");
    let legacy_db = legacy_hub_dir.join("hub.db");
    let new_db = corin_dir.join("corin.db");

    if legacy_db.exists() && !new_db.exists() {
        fs::rename(&legacy_db, &new_db).map_err(|e| ConfigError::Io(e.to_string()))?;

        // Remove the old hub/ directory if it is now empty.
        if legacy_hub_dir.exists()
            && fs::read_dir(&legacy_hub_dir)
                .map_err(|e| ConfigError::Io(e.to_string()))?
                .next()
                .is_none()
        {
            let _ = fs::remove_dir(&legacy_hub_dir);
        }
    }

    Ok(())
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

    // Detect and link Uteke if installed
    // Creates ~/.codecora/uteke/ → ~/.uteke/ symlink
    // Hub reads Uteke data through this symlink (Phase 2 integration)
    if let Err(e) = link_uteke() {
        eprintln!("Warning: failed to link Uteke: {e}");
    }

    let config = if config_path()?.exists() {
        load_config()?
    } else {
        let config = CodecoraConfig::default();
        save_config(&config)?;
        config
    };

    let db_path = corin_db_path()?;
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
    fn test_corin_db_path_ends_with_corin_db() {
        let path = corin_db_path().unwrap();
        assert!(path.ends_with("corin.db"));
    }

    #[test]
    fn test_codecora_root_ends_with_codecora() {
        let path = codecora_root().unwrap();
        assert!(path.ends_with(".codecora"));
    }

    #[test]
    fn test_uteke_symlink_path_ends_with_uteke() {
        let path = uteke_symlink_path().unwrap();
        assert!(path.ends_with("uteke"));
    }

    #[test]
    fn test_expand_tilde() {
        let path = expand_tilde("~/some/dir");
        assert!(!path.starts_with("~"));
        assert!(path.ends_with("some/dir"));
    }

    #[test]
    fn test_expand_tilde_no_tilde() {
        let path = expand_tilde("/absolute/path");
        assert!(path.starts_with("/absolute"));
    }

    #[test]
    fn test_detect_uteke_returns_some_if_installed() {
        // This test only passes if Uteke is actually installed
        // On CI or fresh machines, this will be None — that's fine
        let result = detect_uteke();
        if let Some(ref path) = result {
            assert!(
                path.join("uteke.db").exists(),
                "detect_uteke returned {path:?} but uteke.db not found"
            );
        }
        // None is also valid — no assertion needed
    }
}
