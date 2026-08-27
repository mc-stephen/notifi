use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(tag = "mode")]
pub enum WebhookConfig {
    #[serde(rename = "relay")]
    Relay {
        target_url: String,
        method: String, // e.g., "POST"
        headers: HashMap<String, String>,
        payload_template: String,
    },
    #[serde(rename = "function")]
    Function { function_id: String },
}

impl WebhookConfig {
    pub fn load(brand: &str) -> Result<Self, String> {
        let path = ["configs", brand, "webhook", "config.json"]
            .iter()
            .collect::<std::path::PathBuf>();
        let bytes = std::fs::read(&path).map_err(|e| format!("Failed to read {path:?}: {e}"))?;
        serde_json::from_slice::<Self>(&bytes).map_err(|e| format!("Failed to parse config: {e}"))
    }
}
