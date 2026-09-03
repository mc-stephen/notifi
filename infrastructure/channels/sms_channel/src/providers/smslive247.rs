use serde::Deserialize;

/// SmsLive247 configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct SmsLive247Config {
    pub api_key: String,
    pub sender_id: String,
    #[serde(default)]
    pub base_url: Option<String>,
}
