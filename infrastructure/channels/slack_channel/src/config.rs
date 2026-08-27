use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum SlackConfig {
    #[serde(rename = "webhook")]
    Webhook { webhook_url: String },
    #[serde(rename = "bot")]
    BotApi {
        bot_token: String,
        channel_id: String,
    },
}

impl SlackConfig {
    pub fn load(brand: &str) -> Result<Self, String> {
        let path: PathBuf = ["configs", brand, "slack", "config.json"].iter().collect();
        let bytes = fs::read(&path).map_err(|e| format!("Failed to read {path:?}: {e}"))?;
        serde_json::from_slice::<Self>(&bytes).map_err(|e| format!("Failed to parse config: {e}"))
    }
}
