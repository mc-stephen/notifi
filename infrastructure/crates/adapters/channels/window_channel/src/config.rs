use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum WindowConfig {
    #[serde(rename = "webhook")]
    Webhook {},
    #[serde(rename = "wns")]
    Wns {
        client_id: String,
        client_secret: String,
        package_sid: String,
    },
}

impl WindowConfig {
    pub fn load(brand: &str) -> Result<Self, String> {
        let path: PathBuf = ["configs", brand, "window", "config.json"].iter().collect();
        let bytes = fs::read(&path).map_err(|e| format!("Failed to read {path:?}: {e}"))?;
        serde_json::from_slice::<Self>(&bytes).map_err(|e| format!("Failed to parse config: {e}"))
    }
}
