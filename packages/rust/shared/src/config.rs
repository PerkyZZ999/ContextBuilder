//! Application configuration for ContextBuilder.
//!
//! User config lives at `~/.contextbuilder/contextbuilder.toml`.
//! CLI flags override config file values, which override defaults.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{ContextBuilderError, Result};

/// Default configuration file name.
const CONFIG_FILE_NAME: &str = "contextbuilder.toml";

/// Default config directory name under the user's home.
const CONFIG_DIR_NAME: &str = ".contextbuilder";

// ---------------------------------------------------------------------------
// Config structs (matching contextbuilder.toml schema)
// ---------------------------------------------------------------------------

/// Top-level application config, deserialized from TOML.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    /// Global defaults.
    #[serde(default)]
    pub defaults: DefaultsConfig,

    /// OpenRouter settings.
    #[serde(default)]
    pub openrouter: OpenRouterConfig,

    /// Crawl policies.
    #[serde(default)]
    pub crawl_policies: CrawlPoliciesConfig,

    /// Registered knowledge bases.
    #[serde(default)]
    pub kbs: Vec<KbRegistryEntry>,
}

/// `[defaults]` section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsConfig {
    /// Default KB output directory.
    #[serde(default = "default_output_dir")]
    pub output_dir: String,

    /// Default maximum crawl depth.
    #[serde(default = "default_crawl_depth")]
    pub crawl_depth: u32,

    /// Default concurrent requests.
    #[serde(default = "default_crawl_concurrency")]
    pub crawl_concurrency: u32,

    /// Discovery/crawl mode.
    #[serde(default = "default_mode")]
    pub mode: String,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            output_dir: default_output_dir(),
            crawl_depth: default_crawl_depth(),
            crawl_concurrency: default_crawl_concurrency(),
            mode: default_mode(),
        }
    }
}

fn default_output_dir() -> String {
    "~/contextbuilder-kbs".into()
}
fn default_crawl_depth() -> u32 {
    3
}
fn default_crawl_concurrency() -> u32 {
    4
}
fn default_mode() -> String {
    "auto".into()
}

/// `[openrouter]` section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterConfig {
    /// Name of the env var holding the API key (never store the key itself).
    #[serde(default = "default_api_key_env")]
    pub api_key_env: String,

    /// Default model to use for enrichment.
    #[serde(default = "default_model")]
    pub default_model: String,
}

impl Default for OpenRouterConfig {
    fn default() -> Self {
        Self {
            api_key_env: default_api_key_env(),
            default_model: default_model(),
        }
    }
}

fn default_api_key_env() -> String {
    "OPENROUTER_API_KEY".into()
}
fn default_model() -> String {
    "moonshotai/kimi-k2.5".into()
}

/// `[crawl_policies]` section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlPoliciesConfig {
    /// URL include patterns.
    #[serde(default)]
    pub include_patterns: Vec<String>,

    /// URL exclude patterns.
    #[serde(default)]
    pub exclude_patterns: Vec<String>,

    /// Whether to respect robots.txt.
    #[serde(default = "default_true")]
    pub respect_robots_txt: bool,

    /// Minimum ms between requests to the same host.
    #[serde(default = "default_rate_limit")]
    pub rate_limit_ms: u64,
}

impl Default for CrawlPoliciesConfig {
    fn default() -> Self {
        Self {
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            respect_robots_txt: true,
            rate_limit_ms: default_rate_limit(),
        }
    }
}

fn default_true() -> bool {
    true
}
fn default_rate_limit() -> u64 {
    200
}

/// `[[kbs]]` entry — a registered KB in the config's KB registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KbRegistryEntry {
    /// Human-readable name.
    pub name: String,
    /// Path to the KB directory on disk.
    pub path: String,
    /// Original documentation source URL.
    pub source_url: String,
}

// ---------------------------------------------------------------------------
// Crawl config (runtime, merged from config + CLI flags)
// ---------------------------------------------------------------------------

/// Runtime crawl configuration — merged from config file + CLI flags.
#[derive(Debug, Clone)]
pub struct CrawlConfig {
    /// Maximum crawl depth from the root URL.
    pub depth: u32,
    /// Maximum concurrent HTTP requests.
    pub concurrency: u32,
    /// URL include glob patterns.
    pub include_patterns: Vec<String>,
    /// URL exclude glob patterns.
    pub exclude_patterns: Vec<String>,
    /// Rate limit in ms between requests to the same host.
    pub rate_limit_ms: u64,
    /// Discovery/crawl mode: "auto", "prefer-llms", "crawl-only".
    pub mode: String,
    /// Whether to respect robots.txt.
    pub respect_robots_txt: bool,
}

impl From<&AppConfig> for CrawlConfig {
    fn from(config: &AppConfig) -> Self {
        Self {
            depth: config.defaults.crawl_depth,
            concurrency: config.defaults.crawl_concurrency,
            include_patterns: config.crawl_policies.include_patterns.clone(),
            exclude_patterns: config.crawl_policies.exclude_patterns.clone(),
            rate_limit_ms: config.crawl_policies.rate_limit_ms,
            mode: config.defaults.mode.clone(),
            respect_robots_txt: config.crawl_policies.respect_robots_txt,
        }
    }
}

// ---------------------------------------------------------------------------
// Config loading
// ---------------------------------------------------------------------------

/// Get the path to the config directory (`~/.contextbuilder/`).
pub fn config_dir() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| ContextBuilderError::config("could not determine home directory"))?;
    Ok(home.join(CONFIG_DIR_NAME))
}

/// Get the path to the config file (`~/.contextbuilder/contextbuilder.toml`).
pub fn config_file_path() -> Result<PathBuf> {
    Ok(config_dir()?.join(CONFIG_FILE_NAME))
}

/// Load the application config from disk. Returns defaults if the file does not exist.
pub fn load_config() -> Result<AppConfig> {
    let path = config_file_path()?;

    if !path.exists() {
        tracing::debug!(?path, "config file not found, using defaults");
        return Ok(AppConfig::default());
    }

    load_config_from(&path)
}

/// Load the application config from a specific file path.
pub fn load_config_from(path: &Path) -> Result<AppConfig> {
    let content = std::fs::read_to_string(path).map_err(|e| ContextBuilderError::io(path, e))?;

    toml::from_str(&content).map_err(|e| {
        ContextBuilderError::config(format!("failed to parse {}: {e}", path.display()))
    })
}

/// Create the config directory and write a default config file.
/// Returns the path to the created file.
pub fn init_config() -> Result<PathBuf> {
    let dir = config_dir()?;
    std::fs::create_dir_all(&dir).map_err(|e| ContextBuilderError::io(&dir, e))?;

    let path = dir.join(CONFIG_FILE_NAME);
    let config = AppConfig::default();
    let content =
        toml::to_string_pretty(&config).map_err(|e| ContextBuilderError::config(e.to_string()))?;

    std::fs::write(&path, content).map_err(|e| ContextBuilderError::io(&path, e))?;
    tracing::info!(?path, "created default config file");

    Ok(path)
}

/// Check that the OpenRouter API key env var is set and non-empty.
pub fn validate_api_key(config: &AppConfig) -> Result<()> {
    let var_name = &config.openrouter.api_key_env;
    match std::env::var(var_name) {
        Ok(val) if !val.is_empty() => Ok(()),
        _ => Err(ContextBuilderError::config(format!(
            "OpenRouter API key not found. Set the {var_name} environment variable.\n\
             Get a key at https://openrouter.ai/keys"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_serializes() {
        let config = AppConfig::default();
        let toml_str = toml::to_string_pretty(&config).expect("serialize default config");
        assert!(toml_str.contains("output_dir"));
        assert!(toml_str.contains("OPENROUTER_API_KEY"));
    }

    #[test]
    fn config_roundtrip() {
        let config = AppConfig::default();
        let toml_str = toml::to_string_pretty(&config).expect("serialize");
        let parsed: AppConfig = toml::from_str(&toml_str).expect("deserialize");
        assert_eq!(parsed.defaults.crawl_depth, 3);
        assert_eq!(parsed.openrouter.api_key_env, "OPENROUTER_API_KEY");
    }

    #[test]
    fn config_with_kbs() {
        let toml_str = r#"
[defaults]
output_dir = "/tmp/kbs"

[[kbs]]
name = "test-kb"
path = "/tmp/kbs/test-kb"
source_url = "https://example.com/docs"
"#;
        let config: AppConfig = toml::from_str(toml_str).expect("parse");
        assert_eq!(config.kbs.len(), 1);
        assert_eq!(config.kbs[0].name, "test-kb");
    }

    #[test]
    fn crawl_config_from_app_config() {
        let app = AppConfig::default();
        let crawl = CrawlConfig::from(&app);
        assert_eq!(crawl.depth, 3);
        assert_eq!(crawl.concurrency, 4);
        assert_eq!(crawl.rate_limit_ms, 200);
    }

    #[test]
    fn api_key_validation() {
        let mut config = AppConfig::default();
        // Use a unique env var name to avoid interfering with other tests
        config.openrouter.api_key_env = "CB_TEST_NONEXISTENT_KEY_12345".into();
        let result = validate_api_key(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("API key not found"));
    }
}
