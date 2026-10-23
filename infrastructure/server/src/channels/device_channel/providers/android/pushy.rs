use serde::Deserialize;

/// Pushy configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct PushyConfig {
    pub api_key: String,
    #[serde(default)]
    pub app_id: Option<String>,
}
