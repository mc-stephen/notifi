use serde::de::DeserializeOwned;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur when loading a tenant channel configuration.
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("config directory not found for brand '{brand}' channel '{channel}' at {path}")]
    DirectoryNotFound {
        brand: String,
        channel: String,
        path: PathBuf,
    },
    #[error("I/O error reading config for brand '{brand}' channel '{channel}': {source}")]
    Io {
        brand: String,
        channel: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config for brand '{brand}' channel '{channel}': {message}")]
    Parse {
        brand: String,
        channel: String,
        message: String,
    },
}

/// Resolves the filesystem path for a tenant's channel configuration.
///
/// Canonical layout under the config root (`NOTIFI_CONFIG_ROOT`, defaults to
/// `configs`):
///
/// ```text
/// {root}/brands/{brand}/config/{channel_name}/...      channel configs
/// {root}/brands/{brand}/templates/{template_name}/... brand-scoped templates
/// ```
///
/// The root is the assets root: `brands/` sits directly under it.
pub struct ConfigResolver;

impl ConfigResolver {
    /// Returns the root directory under which all brand dirs live.
    /// Override with the `NOTIFI_CONFIG_ROOT` env var (defaults to `configs`).
    fn root() -> PathBuf {
        std::env::var("NOTIFI_CONFIG_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("configs"))
    }

    /// Builds the channel config path: `{root}/brands/{brand}/config/{channel_name}/`.
    pub fn channel_dir(brand: &str, channel_name: &str) -> PathBuf {
        Self::root()
            .join("brands")
            .join(brand)
            .join("config")
            .join(channel_name)
    }

    /// Builds the template path: `{root}/brands/{brand}/templates/{template_name}/`.
    pub fn template_dir(brand: &str, template_name: &str) -> PathBuf {
        Self::root()
            .join("brands")
            .join(brand)
            .join("templates")
            .join(template_name)
    }

    /// Reads and parses a JSON config file for a tenant channel.
    ///
    /// Path: `{root}/brands/{brand}/config/{channel_name}/{file_name}`.
    ///
    /// Returns [`ConfigError::DirectoryNotFound`] when the directory does not
    /// exist, which lets channels distinguish "no config installed for this
    /// tenant" from a broken config.
    pub fn load_json<T: DeserializeOwned>(
        brand: &str,
        channel_name: &str,
        file_name: &str,
    ) -> Result<T, ConfigError> {
        let dir = Self::channel_dir(brand, channel_name);
        if !dir.is_dir() {
            return Err(ConfigError::DirectoryNotFound {
                brand: brand.to_string(),
                channel: channel_name.to_string(),
                path: dir,
            });
        }

        let path = dir.join(file_name);
        let bytes = fs::read(&path).map_err(|source| ConfigError::Io {
            brand: brand.to_string(),
            channel: channel_name.to_string(),
            source,
        })?;

        serde_json::from_slice(&bytes).map_err(|e| ConfigError::Parse {
            brand: brand.to_string(),
            channel: channel_name.to_string(),
            message: e.to_string(),
        })
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

    #[derive(serde::Deserialize, PartialEq, Debug)]
    struct StubConfig {
        port: u16,
        host: String,
    }

    #[test]
    fn resolves_channel_dir_from_env_root() {
        let _guard = ENV_LOCK.lock().unwrap();
        set_env("NOTIFI_CONFIG_ROOT", "/tmp/notifi-configs");
        let dir = ConfigResolver::channel_dir("acme", "email");
        assert_eq!(
            dir,
            PathBuf::from("/tmp/notifi-configs/brands/acme/config/email")
        );
        clear_env("NOTIFI_CONFIG_ROOT");
    }

    #[test]
    fn resolves_template_dir_from_env_root() {
        let _guard = ENV_LOCK.lock().unwrap();
        set_env("NOTIFI_CONFIG_ROOT", "/tmp/notifi-configs");
        let dir = ConfigResolver::template_dir("acme", "welcome");
        assert_eq!(
            dir,
            PathBuf::from("/tmp/notifi-configs/brands/acme/templates/welcome")
        );
        clear_env("NOTIFI_CONFIG_ROOT");
    }

    #[test]
    fn load_json_reports_missing_directory() {
        let _guard = ENV_LOCK.lock().unwrap();
        set_env("NOTIFI_CONFIG_ROOT", "/nonexistent/notifi");
        let err = ConfigResolver::load_json::<StubConfig>("acme", "email", "config.json")
            .expect_err("should fail");
        assert!(matches!(err, ConfigError::DirectoryNotFound { .. }));
        clear_env("NOTIFI_CONFIG_ROOT");
    }

    #[test]
    fn load_json_parses_file() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = std::env::temp_dir().join("notifi-test-root");
        set_env("NOTIFI_CONFIG_ROOT", root.to_str().unwrap());
        let dir = root
            .join("brands")
            .join("acme")
            .join("config")
            .join("email");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("config.json"),
            r#"{"port": 587, "host": "smtp.example.com"}"#,
        )
        .unwrap();

        let cfg = ConfigResolver::load_json::<StubConfig>("acme", "email", "config.json").unwrap();
        assert_eq!(cfg.host, "smtp.example.com");
        assert_eq!(cfg.port, 587);

        std::fs::remove_dir_all(&root).unwrap();
        clear_env("NOTIFI_CONFIG_ROOT");
    }
}
