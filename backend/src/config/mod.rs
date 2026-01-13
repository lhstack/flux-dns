//! Configuration management module
//!
//! Handles loading configuration from environment variables and config files,
//! with environment variables taking priority over config file values.
//!
//! # Configuration Priority
//!
//! 1. Environment variables (highest priority)
//! 2. Configuration file (config.toml)
//! 3. Default values (lowest priority)
//!
//! # Supported Configuration
//!
//! - Database connection URL (Requirements 6.4)
//! - DNS service listening ports (Requirements 6.5)
//! - Web service listening port (Requirements 6.6)
//! - Upstream DNS server list (Requirements 6.7)
//! - TLS certificate paths (Requirements 6.8)

use std::path::{Path, PathBuf};
use std::sync::RwLock;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    // Service ports
    pub dns_udp_port: u16,
    pub dns_dot_port: u16,
    pub dns_doh_port: u16,
    pub dns_doq_port: u16,
    pub web_port: u16,

    // TLS configuration
    pub tls_cert_path: Option<PathBuf>,
    pub tls_key_path: Option<PathBuf>,

    // Database configuration
    pub database_url: String,

    // Authentication configuration
    pub admin_username: String,
    pub admin_password: String,

    // Log configuration
    pub log_path: PathBuf,
    pub log_level: String,
    pub log_max_size: u64,
    pub log_retention_days: u32,

    // Cache configuration
    pub cache_ttl: u64,
    pub cache_max_entries: usize,

    // Upstream DNS servers
    pub upstream_servers: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            dns_udp_port: 53,
            dns_dot_port: 853,
            dns_doh_port: 443,
            dns_doq_port: 8853,
            web_port: 8080,
            tls_cert_path: None,
            tls_key_path: None,
            database_url: "sqlite:dns_proxy.db?mode=rwc".to_string(),
            admin_username: "admin".to_string(),
            admin_password: "admin".to_string(),
            log_path: PathBuf::from("logs"),
            log_level: "info".to_string(),
            log_max_size: 10 * 1024 * 1024, // 10MB
            log_retention_days: 30,
            cache_ttl: 60,
            cache_max_entries: 10000,
            upstream_servers: vec![
                "8.8.8.8:53".to_string(),
                "8.8.4.4:53".to_string(),
            ],
        }
    }
}

/// Partial configuration for merging from different sources
/// All fields are optional to allow partial configuration from any source
#[derive(Debug, Default, Clone, Deserialize)]
pub struct PartialConfig {
    pub dns_udp_port: Option<u16>,
    pub dns_dot_port: Option<u16>,
    pub dns_doh_port: Option<u16>,
    pub dns_doq_port: Option<u16>,
    pub web_port: Option<u16>,
    pub tls_cert_path: Option<PathBuf>,
    pub tls_key_path: Option<PathBuf>,
    pub database_url: Option<String>,
    pub admin_username: Option<String>,
    pub admin_password: Option<String>,
    pub log_path: Option<PathBuf>,
    pub log_level: Option<String>,
    pub log_max_size: Option<u64>,
    pub log_retention_days: Option<u32>,
    pub cache_ttl: Option<u64>,
    pub cache_max_entries: Option<usize>,
    pub upstream_servers: Option<Vec<String>>,
}

/// Configuration manager responsible for loading and providing access to configuration
///
/// Implements Requirements 6.1-6.8:
/// - 6.1: Support reading from environment variables
/// - 6.2: Support reading from configuration file
/// - 6.3: Environment variables take priority over config file
/// - 6.4: Support database connection URL configuration
/// - 6.5: Support DNS service listening port configuration
/// - 6.6: Support Web service listening port configuration
/// - 6.7: Support upstream DNS server list configuration
/// - 6.8: Support TLS certificate path configuration
pub struct ConfigManager {
    config: RwLock<AppConfig>,
}

impl ConfigManager {
    /// Load configuration from environment variables and config file
    ///
    /// Priority order (highest to lowest):
    /// 1. Environment variables
    /// 2. Config file (config.toml)
    /// 3. Default values
    pub fn load() -> Result<Self> {
        Self::load_with_path("config.toml")
    }

    /// Load configuration with a custom config file path
    pub fn load_with_path<P: AsRef<Path>>(config_path: P) -> Result<Self> {
        // Load .env file if present (for development convenience)
        let _ = dotenvy::dotenv();

        // Start with defaults
        let mut config = AppConfig::default();

        // Load from config file if exists (lower priority)
        if let Ok(file_config) = Self::load_from_file(config_path.as_ref()) {
            Self::merge_config(&mut config, file_config);
        }

        // Load from environment variables (higher priority - applied last)
        let env_config = Self::load_from_env();
        Self::merge_config(&mut config, env_config);

        Ok(Self {
            config: RwLock::new(config),
        })
    }

    /// Create ConfigManager from explicit configs for testing
    /// This allows testing the merge logic with controlled inputs
    pub fn from_configs(
        file_config: Option<PartialConfig>,
        env_config: Option<PartialConfig>,
    ) -> Self {
        let mut config = AppConfig::default();

        // Apply file config first (lower priority)
        if let Some(fc) = file_config {
            Self::merge_config(&mut config, fc);
        }

        // Apply env config last (higher priority)
        if let Some(ec) = env_config {
            Self::merge_config(&mut config, ec);
        }

        Self {
            config: RwLock::new(config),
        }
    }

    /// Get current configuration
    pub fn get(&self) -> AppConfig {
        self.config.read().unwrap().clone()
    }

    /// Load configuration from environment variables
    pub fn load_from_env() -> PartialConfig {
        PartialConfig {
            dns_udp_port: std::env::var("DNS_UDP_PORT")
                .ok()
                .and_then(|v| v.parse().ok()),
            dns_dot_port: std::env::var("DNS_DOT_PORT")
                .ok()
                .and_then(|v| v.parse().ok()),
            dns_doh_port: std::env::var("DNS_DOH_PORT")
                .ok()
                .and_then(|v| v.parse().ok()),
            dns_doq_port: std::env::var("DNS_DOQ_PORT")
                .ok()
                .and_then(|v| v.parse().ok()),
            web_port: std::env::var("WEB_PORT")
                .ok()
                .and_then(|v| v.parse().ok()),
            tls_cert_path: std::env::var("TLS_CERT_PATH").ok().map(PathBuf::from),
            tls_key_path: std::env::var("TLS_KEY_PATH").ok().map(PathBuf::from),
            database_url: std::env::var("DATABASE_URL").ok(),
            admin_username: std::env::var("ADMIN_USERNAME").ok(),
            admin_password: std::env::var("ADMIN_PASSWORD").ok(),
            log_path: std::env::var("LOG_PATH").ok().map(PathBuf::from),
            log_level: std::env::var("LOG_LEVEL").ok(),
            log_max_size: std::env::var("LOG_MAX_SIZE")
                .ok()
                .and_then(|v| v.parse().ok()),
            log_retention_days: std::env::var("LOG_RETENTION_DAYS")
                .ok()
                .and_then(|v| v.parse().ok()),
            cache_ttl: std::env::var("CACHE_TTL")
                .ok()
                .and_then(|v| v.parse().ok()),
            cache_max_entries: std::env::var("CACHE_MAX_ENTRIES")
                .ok()
                .and_then(|v| v.parse().ok()),
            upstream_servers: std::env::var("UPSTREAM_SERVERS")
                .ok()
                .map(|v| v.split(',').map(|s| s.trim().to_string()).collect()),
        }
    }

    /// Load configuration from TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<PartialConfig> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;
        let config: PartialConfig =
            toml::from_str(&content).with_context(|| "Failed to parse config file as TOML")?;
        Ok(config)
    }

    /// Merge partial config into full config
    /// Values from partial config override existing values in config
    pub fn merge_config(config: &mut AppConfig, partial: PartialConfig) {
        if let Some(v) = partial.dns_udp_port {
            config.dns_udp_port = v;
        }
        if let Some(v) = partial.dns_dot_port {
            config.dns_dot_port = v;
        }
        if let Some(v) = partial.dns_doh_port {
            config.dns_doh_port = v;
        }
        if let Some(v) = partial.dns_doq_port {
            config.dns_doq_port = v;
        }
        if let Some(v) = partial.web_port {
            config.web_port = v;
        }
        if let Some(v) = partial.tls_cert_path {
            config.tls_cert_path = Some(v);
        }
        if let Some(v) = partial.tls_key_path {
            config.tls_key_path = Some(v);
        }
        if let Some(v) = partial.database_url {
            config.database_url = v;
        }
        if let Some(v) = partial.admin_username {
            config.admin_username = v;
        }
        if let Some(v) = partial.admin_password {
            config.admin_password = v;
        }
        if let Some(v) = partial.log_path {
            config.log_path = v;
        }
        if let Some(v) = partial.log_level {
            config.log_level = v;
        }
        if let Some(v) = partial.log_max_size {
            config.log_max_size = v;
        }
        if let Some(v) = partial.log_retention_days {
            config.log_retention_days = v;
        }
        if let Some(v) = partial.cache_ttl {
            config.cache_ttl = v;
        }
        if let Some(v) = partial.cache_max_entries {
            config.cache_max_entries = v;
        }
        if let Some(v) = partial.upstream_servers {
            config.upstream_servers = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    use proptest::prelude::*;

    // Feature: dns-proxy-service, Property 9: 配置优先级正确性
    // *For any* 同时存在于环境变量和配置文件中的配置项，Config_Manager 应始终返回环境变量的值。
    // **Validates: Requirements 6.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_config_priority_env_over_file(
            // Generate random port values for testing
            file_dns_udp_port in 1024u16..65535,
            env_dns_udp_port in 1024u16..65535,
            file_web_port in 1024u16..65535,
            env_web_port in 1024u16..65535,
            file_cache_ttl in 1u64..3600,
            env_cache_ttl in 1u64..3600,
            file_log_level in "(debug|info|warn|error)",
            env_log_level in "(debug|info|warn|error)",
            file_database_url in "[a-z]{5,10}",
            env_database_url in "[a-z]{5,10}",
        ) {
            // Create file config with generated values
            let file_config = PartialConfig {
                dns_udp_port: Some(file_dns_udp_port),
                web_port: Some(file_web_port),
                cache_ttl: Some(file_cache_ttl),
                log_level: Some(file_log_level.clone()),
                database_url: Some(format!("sqlite:{}.db", file_database_url)),
                ..Default::default()
            };

            // Create env config with generated values
            let env_config = PartialConfig {
                dns_udp_port: Some(env_dns_udp_port),
                web_port: Some(env_web_port),
                cache_ttl: Some(env_cache_ttl),
                log_level: Some(env_log_level.clone()),
                database_url: Some(format!("sqlite:{}.db", env_database_url)),
                ..Default::default()
            };

            // Create ConfigManager with both configs (file first, then env)
            let manager = ConfigManager::from_configs(Some(file_config), Some(env_config));
            let config = manager.get();

            // Property: Environment variables should ALWAYS take priority over file config
            // When both are set, the result should equal the env value
            prop_assert_eq!(config.dns_udp_port, env_dns_udp_port,
                "dns_udp_port: env value {} should override file value {}",
                env_dns_udp_port, file_dns_udp_port);

            prop_assert_eq!(config.web_port, env_web_port,
                "web_port: env value {} should override file value {}",
                env_web_port, file_web_port);

            prop_assert_eq!(config.cache_ttl, env_cache_ttl,
                "cache_ttl: env value {} should override file value {}",
                env_cache_ttl, file_cache_ttl);

            prop_assert_eq!(config.log_level, env_log_level.clone(),
                "log_level: env value {} should override file value {}",
                env_log_level, file_log_level);

            prop_assert_eq!(config.database_url, format!("sqlite:{}.db", env_database_url),
                "database_url: env value should override file value");
        }

        #[test]
        fn test_config_priority_file_used_when_env_absent(
            // Generate random values for file config
            file_dns_udp_port in 1024u16..65535,
            file_web_port in 1024u16..65535,
            file_cache_ttl in 1u64..3600,
            file_log_level in "(debug|info|warn|error)",
        ) {
            // Create file config with values
            let file_config = PartialConfig {
                dns_udp_port: Some(file_dns_udp_port),
                web_port: Some(file_web_port),
                cache_ttl: Some(file_cache_ttl),
                log_level: Some(file_log_level.clone()),
                ..Default::default()
            };

            // Create empty env config (no values set)
            let env_config = PartialConfig::default();

            // Create ConfigManager with file config and empty env config
            let manager = ConfigManager::from_configs(Some(file_config), Some(env_config));
            let config = manager.get();

            // Property: When env is not set, file config values should be used
            prop_assert_eq!(config.dns_udp_port, file_dns_udp_port,
                "dns_udp_port: file value {} should be used when env is absent",
                file_dns_udp_port);

            prop_assert_eq!(config.web_port, file_web_port,
                "web_port: file value {} should be used when env is absent",
                file_web_port);

            prop_assert_eq!(config.cache_ttl, file_cache_ttl,
                "cache_ttl: file value {} should be used when env is absent",
                file_cache_ttl);

            prop_assert_eq!(config.log_level, file_log_level.clone(),
                "log_level: file value {} should be used when env is absent",
                file_log_level);
        }

        #[test]
        fn test_config_priority_partial_env_override(
            // File has all values
            file_dns_udp_port in 1024u16..65535,
            file_web_port in 1024u16..65535,
            file_cache_ttl in 1u64..3600,
            // Env only has some values
            env_web_port in 1024u16..65535,
        ) {
            // Create file config with all values
            let file_config = PartialConfig {
                dns_udp_port: Some(file_dns_udp_port),
                web_port: Some(file_web_port),
                cache_ttl: Some(file_cache_ttl),
                ..Default::default()
            };

            // Create env config with only web_port set
            let env_config = PartialConfig {
                web_port: Some(env_web_port),
                ..Default::default()
            };

            let manager = ConfigManager::from_configs(Some(file_config), Some(env_config));
            let config = manager.get();

            // Property: Only the env values that are set should override file values
            // web_port should use env value
            prop_assert_eq!(config.web_port, env_web_port,
                "web_port: env value {} should override file value {}",
                env_web_port, file_web_port);

            // dns_udp_port and cache_ttl should use file values (not in env)
            prop_assert_eq!(config.dns_udp_port, file_dns_udp_port,
                "dns_udp_port: file value {} should be used when env is absent",
                file_dns_udp_port);

            prop_assert_eq!(config.cache_ttl, file_cache_ttl,
                "cache_ttl: file value {} should be used when env is absent",
                file_cache_ttl);
        }
    }

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.dns_udp_port, 53);
        assert_eq!(config.dns_dot_port, 853);
        assert_eq!(config.dns_doh_port, 443);
        assert_eq!(config.dns_doq_port, 8853);
        assert_eq!(config.web_port, 8080);
        assert_eq!(config.database_url, "sqlite:dns_proxy.db?mode=rwc");
        assert_eq!(config.admin_username, "admin");
        assert_eq!(config.log_level, "info");
        assert_eq!(config.cache_ttl, 60);
        assert!(!config.upstream_servers.is_empty());
    }

    #[test]
    fn test_load_from_toml_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
dns_udp_port = 5353
web_port = 9090
database_url = "sqlite:test.db"
admin_username = "testuser"
log_level = "debug"
upstream_servers = ["1.1.1.1:53", "1.0.0.1:53"]
"#
        )
        .unwrap();

        let config = ConfigManager::load_from_file(file.path()).unwrap();
        assert_eq!(config.dns_udp_port, Some(5353));
        assert_eq!(config.web_port, Some(9090));
        assert_eq!(config.database_url, Some("sqlite:test.db".to_string()));
        assert_eq!(config.admin_username, Some("testuser".to_string()));
        assert_eq!(config.log_level, Some("debug".to_string()));
        assert_eq!(
            config.upstream_servers,
            Some(vec!["1.1.1.1:53".to_string(), "1.0.0.1:53".to_string()])
        );
    }

    #[test]
    fn test_merge_config() {
        let mut config = AppConfig::default();
        let partial = PartialConfig {
            dns_udp_port: Some(5353),
            web_port: Some(9090),
            database_url: Some("sqlite:merged.db".to_string()),
            ..Default::default()
        };

        ConfigManager::merge_config(&mut config, partial);

        assert_eq!(config.dns_udp_port, 5353);
        assert_eq!(config.web_port, 9090);
        assert_eq!(config.database_url, "sqlite:merged.db");
        // Unchanged values should remain default
        assert_eq!(config.dns_dot_port, 853);
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_env_priority_over_file() {
        // Simulates: file has web_port=9090, env has web_port=8888
        // Result should be 8888 (env wins)
        let file_config = PartialConfig {
            web_port: Some(9090),
            database_url: Some("sqlite:file.db".to_string()),
            ..Default::default()
        };

        let env_config = PartialConfig {
            web_port: Some(8888),
            // database_url not set in env
            ..Default::default()
        };

        let manager = ConfigManager::from_configs(Some(file_config), Some(env_config));
        let config = manager.get();

        // Env value should win for web_port
        assert_eq!(config.web_port, 8888);
        // File value should be used for database_url (not in env)
        assert_eq!(config.database_url, "sqlite:file.db");
    }

    #[test]
    fn test_config_with_file_path() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
dns_udp_port = 5353
cache_ttl = 120
"#
        )
        .unwrap();

        let manager = ConfigManager::load_with_path(file.path()).unwrap();
        let config = manager.get();

        assert_eq!(config.dns_udp_port, 5353);
        assert_eq!(config.cache_ttl, 120);
    }

    #[test]
    fn test_missing_config_file_uses_defaults() {
        // When config file doesn't exist, should use defaults
        let manager = ConfigManager::load_with_path("nonexistent_config.toml").unwrap();
        let config = manager.get();

        assert_eq!(config.dns_udp_port, 53);
        assert_eq!(config.web_port, 8080);
    }

    #[test]
    fn test_partial_config_preserves_unset_values() {
        let file_config = PartialConfig {
            dns_udp_port: Some(5353),
            // All other fields are None
            ..Default::default()
        };

        let manager = ConfigManager::from_configs(Some(file_config), None);
        let config = manager.get();

        // Set value from file
        assert_eq!(config.dns_udp_port, 5353);
        // All other values should be defaults
        assert_eq!(config.dns_dot_port, 853);
        assert_eq!(config.web_port, 8080);
        assert_eq!(config.database_url, "sqlite:dns_proxy.db?mode=rwc");
    }

    #[test]
    fn test_tls_config() {
        let partial = PartialConfig {
            tls_cert_path: Some(PathBuf::from("/path/to/cert.pem")),
            tls_key_path: Some(PathBuf::from("/path/to/key.pem")),
            ..Default::default()
        };

        let manager = ConfigManager::from_configs(Some(partial), None);
        let config = manager.get();

        assert_eq!(
            config.tls_cert_path,
            Some(PathBuf::from("/path/to/cert.pem"))
        );
        assert_eq!(config.tls_key_path, Some(PathBuf::from("/path/to/key.pem")));
    }

    #[test]
    fn test_upstream_servers_config() {
        let partial = PartialConfig {
            upstream_servers: Some(vec![
                "1.1.1.1:53".to_string(),
                "8.8.8.8:53".to_string(),
            ]),
            ..Default::default()
        };

        let manager = ConfigManager::from_configs(Some(partial), None);
        let config = manager.get();

        assert_eq!(config.upstream_servers.len(), 2);
        assert_eq!(config.upstream_servers[0], "1.1.1.1:53");
        assert_eq!(config.upstream_servers[1], "8.8.8.8:53");
    }
}
