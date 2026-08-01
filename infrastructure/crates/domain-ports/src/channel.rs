use notifi_core::config::{ConfigError, ConfigResolver};
use std::path::PathBuf;

/// A channel implementation provides its own config type and parsing logic.
///
/// The kernel only knows the directory convention
/// (`{NOTIFI_CONFIG_ROOT}/brands/{brand}/config/{channel_name}/`); each channel
/// owns how it reads and validates its configuration files via
/// [`ConfigResolver::load_json`].
pub trait ChannelConfigLoader {
    /// The structured config type this channel parses into.
    type Config;

    /// Unique channel identifier (e.g. `"email"`, `"fcm"`, `"whatsapp"`).
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
