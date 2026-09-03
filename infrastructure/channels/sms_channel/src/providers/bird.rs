use serde::Deserialize;

/// Bird SMS configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct BirdConfig {
    pub api_key: String,
    pub base_url: String,
    pub sender_id: String,
}
