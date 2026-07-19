use async_trait::async_trait;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur when loading a channel configuration.
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
pub struct ConfigResolver;

impl ConfigResolver {
    /// Returns the root directory under which all tenant config dirs live.
    /// Override with the `NOTIFI_CONFIG_ROOT` env var (defaults to `configs`).
    fn root() -> PathBuf {
        std::env::var("NOTIFI_CONFIG_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("configs"))
    }

    /// Builds the path: `{root}/{brand}/{channel_name}/`
    pub fn channel_dir(brand: &str, channel_name: &str) -> PathBuf {
        Self::root().join(brand).join(channel_name)
    }
}

/// A channel implementation provides its own config type and parsing logic.
///
/// The core only knows the directory convention; each channel owns how it
/// reads and validates its configuration files.
pub trait ChannelConfigLoader {
    /// The structured config type this channel parses into.
    type Config;

    /// Unique channel identifier (e.g. `"email"`, `"fcm"`).
    fn channel_name() -> &'static str;

    /// Absolute path to the config directory for this channel + tenant.
    fn config_dir(brand: &str) -> PathBuf {
        ConfigResolver::channel_dir(brand, Self::channel_name())
    }

    /// Load and parse configuration for a given tenant brand.
    ///
    /// Implementors should call `Self::config_dir(brand)` to locate their
    /// files, then read and parse them into `Self::Config`.
    fn load_config(brand: &str) -> Result<Self::Config, ConfigError>;
}

/// A notification channel that can send messages.
#[async_trait]
pub trait NotificationChannel: Send + Sync {
    async fn send(&self, recipient: &str, message: &str) -> Result<(), String>;
}
