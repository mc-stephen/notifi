//! Layered application configuration.
//!
//! Layering: compiled defaults ← optional config file (`NOTIFI_CONFIG_FILE`)
//! ← environment overrides (`NOTIFI_*`). Secrets are never configured here —
//! they come from the environment or mounted secret files only.

use serde::Deserialize;

/// Server listen settings.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

/// Logging settings.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LogConfig {
    /// Tracing filter (e.g. `info`, `notifi_core=debug,tower_http=warn`).
    #[serde(default = "default_log_level")]
    pub level: String,
}

/// Postgres connection settings. Optional: when unset, the API boots without
/// a database (readiness reports 503) and migrations are not run.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct DatabaseConfig {
    /// e.g. `postgres://user:pass@localhost:5432/notifi`.
    #[serde(default)]
    pub url: Option<String>,
}

/// Redis connection settings. Optional: when unset, the API boots without
/// Redis (readiness reports 503).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct RedisConfig {
    /// e.g. `redis://:password@localhost:6379`.
    #[serde(default)]
    pub url: Option<String>,
}

/// Root application configuration.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub log: LogConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub redis: RedisConfig,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

impl AppConfig {
    /// Loads configuration from file + environment.
    ///
    /// 1. Starts from compiled defaults.
    /// 2. Overlays `NOTIFI_CONFIG_FILE` (JSON, optional).
    /// 3. Overlays `NOTIFI_HOST`, `NOTIFI_PORT`, `NOTIFI_LOG`,
    ///    `NOTIFI_DATABASE_URL`, `NOTIFI_REDIS_URL`.
    pub fn from_env() -> Result<Self, String> {
        let mut config = Self::default();

        if let Ok(path) = std::env::var("NOTIFI_CONFIG_FILE") {
            let bytes = std::fs::read(&path).map_err(|e| format!("failed to read {path}: {e}"))?;
            let file: AppConfig =
                serde_json::from_slice(&bytes).map_err(|e| format!("invalid config file: {e}"))?;
            config.merge(file);
        }

        if let Ok(host) = std::env::var("NOTIFI_HOST") {
            config.server.host = host;
        }
        if let Ok(port) = std::env::var("NOTIFI_PORT") {
            config.server.port = port
                .parse()
                .map_err(|e| format!("invalid NOTIFI_PORT: {e}"))?;
        }
        if let Ok(level) = std::env::var("NOTIFI_LOG") {
            config.log.level = level;
        }
        if let Ok(url) = std::env::var("NOTIFI_DATABASE_URL") {
            config.database.url = Some(url);
        }
        if let Ok(url) = std::env::var("NOTIFI_REDIS_URL") {
            config.redis.url = Some(url);
        }

        Ok(config)
    }

    fn merge(&mut self, other: AppConfig) {
        if other.server != ServerConfig::default() {
            self.server = other.server;
        }
        if other.log != LogConfig::default() {
            self.log = other.log;
        }
        if other.database != DatabaseConfig::default() {
            self.database = other.database;
        }
        if other.redis != RedisConfig::default() {
            self.redis = other.redis;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serializes tests that mutate process env vars.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn set_env(key: &str, value: &str) {
        // SAFETY: process-local mutation, guarded by ENV_LOCK so tests never race.
        unsafe { std::env::set_var(key, value) };
    }

    fn clear_env(key: &str) {
        // SAFETY: process-local mutation, guarded by ENV_LOCK so tests never race.
        unsafe { std::env::remove_var(key) };
    }

    #[test]
    fn defaults_apply_without_env() {
        let config = AppConfig::default();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.log.level, "info");
    }

    #[test]
    fn env_overrides_apply() {
        let _guard = ENV_LOCK.lock().unwrap();
        set_env("NOTIFI_HOST", "0.0.0.0");
        set_env("NOTIFI_PORT", "9000");
        set_env("NOTIFI_LOG", "debug");
        set_env("NOTIFI_DATABASE_URL", "postgres://u:p@h:5432/notifi");
        set_env("NOTIFI_REDIS_URL", "redis://h:6379");

        let config = AppConfig::from_env().unwrap();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 9000);
        assert_eq!(config.log.level, "debug");
        assert_eq!(
            config.database.url.as_deref(),
            Some("postgres://u:p@h:5432/notifi")
        );
        assert_eq!(config.redis.url.as_deref(), Some("redis://h:6379"));

        clear_env("NOTIFI_HOST");
        clear_env("NOTIFI_PORT");
        clear_env("NOTIFI_LOG");
        clear_env("NOTIFI_DATABASE_URL");
        clear_env("NOTIFI_REDIS_URL");
    }

    #[test]
    fn db_and_redis_are_optional_by_default() {
        let config = AppConfig::default();
        assert_eq!(config.database.url, None);
        assert_eq!(config.redis.url, None);
    }

    #[test]
    fn invalid_port_is_rejected() {
        let _guard = ENV_LOCK.lock().unwrap();
        set_env("NOTIFI_PORT", "not-a-port");
        assert!(AppConfig::from_env().is_err());
        clear_env("NOTIFI_PORT");
    }
}
