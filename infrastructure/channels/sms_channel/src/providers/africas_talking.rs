use serde::Deserialize;

/// Africa's Talking SMS configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct AfricasTalkingConfig {
    pub api_key: String,
    pub username: String,
    pub sender_id: String,
    #[serde(default)]
    pub base_url: Option<String>,
}
