//! Configuration management for RDAP client
//!
//! Configuration priority (highest to lowest):
//! 1. ~/.config/rdap/*.local.json (user local overrides, never updated)
//! 2. ~/.config/rdap/*.json (downloaded configs)
//! 3. /etc/rdap/*.json (system configs)
//! 4. Built-in defaults (embedded in binary)

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use url::Url;

/// Built-in default config (embedded from config/config.json)
const BUILTIN_CONFIG: &str = include_str!("../config/config.json");

/// Built-in TLD overrides (embedded from config/tlds.json)
const BUILTIN_TLDS: &str = include_str!("../config/tlds.json");

/// Built-in IANA TLD list (embedded from config/tlds.txt)
const BUILTIN_TLD_LIST: &str = include_str!("../config/tlds.txt");

/// GitHub raw URLs for config updates
pub const CONFIG_UPDATE_URL: &str =
    "https://raw.githubusercontent.com/xtomcom/rdap/main/config/config.json";
pub const TLDS_UPDATE_URL: &str =
    "https://raw.githubusercontent.com/xtomcom/rdap/main/config/tlds.json";
pub const TLD_LIST_UPDATE_URL: &str = "https://data.iana.org/TLD/tlds-alpha-by-domain.txt";

/// IANA RDAP server for TLD queries
pub const IANA_RDAP_URL: &str = "https://rdap.iana.org/";

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub bootstrap: BootstrapConfig,
    pub cache: CacheConfig,
}

/// Bootstrap URLs configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapConfig {
    pub dns: String,
    pub asn: String,
    pub ipv4: String,
    pub ipv6: String,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub ttl_seconds: u64,
}

/// TLD overrides - maps TLD/SLD to RDAP server URL
pub type TldOverrides = HashMap<String, String>;

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            dns: "https://data.iana.org/rdap/dns.json".to_string(),
            asn: "https://data.iana.org/rdap/asn.json".to_string(),
            ipv4: "https://data.iana.org/rdap/ipv4.json".to_string(),
            ipv6: "https://data.iana.org/rdap/ipv6.json".to_string(),
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl_seconds: 86400, // 24 hours
        }
    }
}

/// Get the user config directory path (~/.config/rdap/)
pub fn user_config_dir() -> Result<PathBuf> {
    let dir = std::env::var("HOME")
        .map(|h| PathBuf::from(h).join(".config/rdap"))
        .unwrap_or_else(|_| PathBuf::from(".config/rdap"));
    Ok(dir)
}

/// Get the system config directory path (/etc/rdap/)
pub fn system_config_dir() -> PathBuf {
    PathBuf::from("/etc/rdap")
}

impl Config {
    /// Get the config directory path (alias for user_config_dir)
    pub fn config_dir() -> Result<PathBuf> {
        user_config_dir()
    }

    /// Load config with priority: local > user > system > builtin
    pub fn load() -> Result<Self> {
        let user_dir = user_config_dir()?;
        let system_dir = system_config_dir();

        // Try config.local.json first (user local overrides)
        let local_path = user_dir.join("config.local.json");
        if local_path.exists()
            && let Ok(content) = fs::read_to_string(&local_path)
            && let Ok(config) = serde_json::from_str(&content)
        {
            log::debug!("Loaded config from {}", local_path.display());
            return Ok(config);
        }

        // Try config.json in user dir (downloaded config)
        let user_path = user_dir.join("config.json");
        if user_path.exists()
            && let Ok(content) = fs::read_to_string(&user_path)
            && let Ok(config) = serde_json::from_str(&content)
        {
            log::debug!("Loaded config from {}", user_path.display());
            return Ok(config);
        }

        // Try system config
        let system_path = system_dir.join("config.json");
        if system_path.exists()
            && let Ok(content) = fs::read_to_string(&system_path)
            && let Ok(config) = serde_json::from_str(&content)
        {
            log::debug!("Loaded config from {}", system_path.display());
            return Ok(config);
        }

        // Fall back to built-in default
        log::debug!("Using built-in config");
        let config: Config = serde_json::from_str(BUILTIN_CONFIG)?;
        Ok(config)
    }

    /// Save config to user config file
    pub fn save(&self) -> Result<()> {
        let config_dir = user_config_dir()?;
        fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("config.json");
        let content = serde_json::to_string_pretty(self)?;
        fs::write(config_path, content)?;

        Ok(())
    }
}

/// Load TLD overrides with priority: local > user > system > builtin
/// Also merges local overrides on top of base config
pub fn load_tld_overrides() -> Result<TldOverrides> {
    let user_dir = user_config_dir()?;
    let system_dir = system_config_dir();

    // Start with base overrides (user > system > builtin)
    let mut overrides = load_base_tld_overrides(&user_dir, &system_dir)?;

    // Merge local overrides on top (tlds.local.json)
    let local_path = user_dir.join("tlds.local.json");
    if local_path.exists()
        && let Ok(content) = fs::read_to_string(&local_path)
        && let Ok(local_overrides) = serde_json::from_str::<TldOverrides>(&content)
    {
        log::debug!(
            "Merging {} local TLD overrides from {}",
            local_overrides.len(),
            local_path.display()
        );
        for (k, v) in local_overrides {
            overrides.insert(k, v);
        }
    }

    Ok(overrides)
}

/// Load base TLD overrides (without local merge)
fn load_base_tld_overrides(
    user_dir: &std::path::Path,
    system_dir: &std::path::Path,
) -> Result<TldOverrides> {
    // Try user config (downloaded)
    let user_path = user_dir.join("tlds.json");
    if user_path.exists()
        && let Ok(content) = fs::read_to_string(&user_path)
        && let Ok(overrides) = serde_json::from_str(&content)
    {
        log::debug!("Loaded TLD overrides from {}", user_path.display());
        return Ok(overrides);
    }

    // Try system config
    let system_path = system_dir.join("tlds.json");
    if system_path.exists()
        && let Ok(content) = fs::read_to_string(&system_path)
        && let Ok(overrides) = serde_json::from_str(&content)
    {
        log::debug!("Loaded TLD overrides from {}", system_path.display());
        return Ok(overrides);
    }

    // Fall back to built-in default
    log::debug!("Using built-in TLD overrides");
    let overrides: TldOverrides = serde_json::from_str(BUILTIN_TLDS)?;
    Ok(overrides)
}

/// Save TLD overrides to user config file
pub fn save_tld_overrides(overrides: &TldOverrides) -> Result<()> {
    let config_dir = user_config_dir()?;
    fs::create_dir_all(&config_dir)?;

    let tlds_path = config_dir.join("tlds.json");
    let content = serde_json::to_string_pretty(overrides)?;
    fs::write(tlds_path, content)?;

    Ok(())
}

/// Look up RDAP server URL for a domain from TLD overrides
pub fn lookup_tld_override(overrides: &TldOverrides, domain: &str) -> Option<Url> {
    let domain = domain.trim_end_matches('.').to_lowercase();
    let parts: Vec<&str> = domain.split('.').collect();

    // Try from most specific to least specific
    // e.g., for "foo.com.af", try "foo.com.af", "com.af", "af"
    for i in 0..parts.len() {
        let suffix = parts[i..].join(".");
        if let Some(url_str) = overrides.get(&suffix)
            && let Ok(url) = Url::parse(url_str)
        {
            return Some(url);
        }
    }

    None
}

/// Update configuration files from GitHub
pub async fn update_configs() -> Result<UpdateResult> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("rdap-rust/", env!("CARGO_PKG_VERSION")))
        .build()?;

    let config_dir = user_config_dir()?;
    fs::create_dir_all(&config_dir)?;

    let mut result = UpdateResult::default();

    // Update config.json
    match client.get(CONFIG_UPDATE_URL).send().await {
        Ok(response) if response.status().is_success() => {
            match response.text().await {
                Ok(content) => {
                    // Validate JSON before saving
                    if serde_json::from_str::<Config>(&content).is_ok() {
                        let path = config_dir.join("config.json");
                        fs::write(&path, &content)?;
                        result.config_updated = true;
                        log::info!("Updated config.json");
                    } else {
                        result.config_error = Some("Invalid config.json format".to_string());
                    }
                }
                Err(e) => result.config_error = Some(format!("Failed to read response: {}", e)),
            }
        }
        Ok(response) => result.config_error = Some(format!("HTTP {}", response.status())),
        Err(e) => result.config_error = Some(format!("Request failed: {}", e)),
    }

    // Update tlds.json
    match client.get(TLDS_UPDATE_URL).send().await {
        Ok(response) if response.status().is_success() => {
            match response.text().await {
                Ok(content) => {
                    // Validate JSON before saving
                    if serde_json::from_str::<TldOverrides>(&content).is_ok() {
                        let path = config_dir.join("tlds.json");
                        fs::write(&path, &content)?;
                        result.tlds_updated = true;
                        log::info!("Updated tlds.json");
                    } else {
                        result.tlds_error = Some("Invalid tlds.json format".to_string());
                    }
                }
                Err(e) => result.tlds_error = Some(format!("Failed to read response: {}", e)),
            }
        }
        Ok(response) => result.tlds_error = Some(format!("HTTP {}", response.status())),
        Err(e) => result.tlds_error = Some(format!("Request failed: {}", e)),
    }

    // Update tlds.txt (IANA TLD list)
    match client.get(TLD_LIST_UPDATE_URL).send().await {
        Ok(response) if response.status().is_success() => {
            match response.text().await {
                Ok(content) => {
                    // Validate content (should have lines starting with letters)
                    let has_valid_tlds = content
                        .lines()
                        .filter(|l| !l.starts_with('#') && !l.is_empty())
                        .take(5)
                        .all(|l| l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
                    if has_valid_tlds {
                        let path = config_dir.join("tlds.txt");
                        fs::write(&path, &content)?;
                        result.tld_list_updated = true;
                        log::info!("Updated tlds.txt");
                    } else {
                        result.tld_list_error = Some("Invalid tlds.txt format".to_string());
                    }
                }
                Err(e) => result.tld_list_error = Some(format!("Failed to read response: {}", e)),
            }
        }
        Ok(response) => result.tld_list_error = Some(format!("HTTP {}", response.status())),
        Err(e) => result.tld_list_error = Some(format!("Request failed: {}", e)),
    }

    Ok(result)
}

/// Result of config update operation
#[derive(Debug, Default)]
pub struct UpdateResult {
    pub config_updated: bool,
    pub config_error: Option<String>,
    pub tlds_updated: bool,
    pub tlds_error: Option<String>,
    pub tld_list_updated: bool,
    pub tld_list_error: Option<String>,
}

/// IANA TLD list - all valid top-level domains
pub struct TldList {
    tlds: std::collections::HashSet<String>,
}

impl TldList {
    /// Load TLD list with priority: user > system > builtin
    pub fn load() -> Result<Self> {
        let user_dir = user_config_dir()?;
        let system_dir = system_config_dir();

        // Try user config
        let user_path = user_dir.join("tlds.txt");
        if user_path.exists()
            && let Ok(content) = fs::read_to_string(&user_path)
        {
            log::debug!("Loaded TLD list from {}", user_path.display());
            return Ok(Self::parse(&content));
        }

        // Try system config
        let system_path = system_dir.join("tlds.txt");
        if system_path.exists()
            && let Ok(content) = fs::read_to_string(&system_path)
        {
            log::debug!("Loaded TLD list from {}", system_path.display());
            return Ok(Self::parse(&content));
        }

        // Fall back to built-in
        log::debug!("Using built-in TLD list");
        Ok(Self::parse(BUILTIN_TLD_LIST))
    }

    /// Parse TLD list from text content
    fn parse(content: &str) -> Self {
        let tlds: std::collections::HashSet<String> = content
            .lines()
            .filter(|line| !line.starts_with('#') && !line.is_empty())
            .map(|line| line.trim().to_lowercase())
            .collect();
        Self { tlds }
    }

    /// Check if a string is a valid TLD
    pub fn is_tld(&self, query: &str) -> bool {
        self.tlds.contains(&query.to_lowercase())
    }

    /// Get the number of TLDs in the list
    pub fn len(&self) -> usize {
        self.tlds.len()
    }

    /// Check if the list is empty
    pub fn is_empty(&self) -> bool {
        self.tlds.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_config_valid() {
        let config: Config = serde_json::from_str(BUILTIN_CONFIG).expect("Invalid builtin config");
        assert!(!config.bootstrap.dns.is_empty());
    }

    #[test]
    fn test_builtin_tlds_valid() {
        let tlds: TldOverrides = serde_json::from_str(BUILTIN_TLDS).expect("Invalid builtin tlds");
        assert!(tlds.contains_key("io"));
        assert!(tlds.contains_key("com.af"));
    }

    #[test]
    fn test_tld_lookup() {
        let overrides: TldOverrides = serde_json::from_str(BUILTIN_TLDS).unwrap();

        // Direct TLD match
        let url = lookup_tld_override(&overrides, "example.io");
        assert!(url.is_some());
        assert!(url.unwrap().as_str().contains("identitydigital"));

        // SLD match
        let url = lookup_tld_override(&overrides, "test.com.af");
        assert!(url.is_some());
        assert!(url.unwrap().as_str().contains("coccaregistry"));

        // No match
        let url = lookup_tld_override(&overrides, "example.com");
        assert!(url.is_none());
    }

    #[test]
    fn test_tld_list_valid() {
        let tld_list = TldList::parse(BUILTIN_TLD_LIST);
        assert!(tld_list.len() > 1000); // Should have 1000+ TLDs
        assert!(tld_list.is_tld("com"));
        assert!(tld_list.is_tld("COM")); // Case insensitive
        assert!(tld_list.is_tld("google"));
        assert!(tld_list.is_tld("io"));
        assert!(!tld_list.is_tld("notarealtld123"));
    }
}
