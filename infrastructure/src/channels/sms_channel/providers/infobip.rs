use serde::Deserialize;

/// Infobip SMS configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct InfobipConfig {
    pub api_key: String,
    pub base_url: String,
    pub sender_id: String,
}
