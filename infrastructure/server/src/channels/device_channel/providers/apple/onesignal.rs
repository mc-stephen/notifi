use serde::Deserialize;

/// OneSignal configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct OneSignalConfig {
    pub app_id: String,
    pub api_key: String,
    #[serde(default)]
    pub rest_api_key: Option<String>,
}
