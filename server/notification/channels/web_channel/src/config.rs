use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct WebConfig {
    pub vapid_public_key: String,
    pub vapid_private_key: String,
    pub contact_email: String,
}

impl WebConfig {
    pub fn load(brand: &str) -> Result<Self, String> {
        let path: PathBuf = ["configs", brand, "web", "config.json"].iter().collect();
        let bytes = fs::read(&path).map_err(|e| format!("Failed to read {path:?}: {e}"))?;
        serde_json::from_slice::<Self>(&bytes).map_err(|e| format!("Failed to parse config: {e}"))
    }
}
