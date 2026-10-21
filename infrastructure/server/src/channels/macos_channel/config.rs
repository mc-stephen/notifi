use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct MacosConfig {
    pub key_id: String,
    pub team_id: String,
    pub bundle_id: String,
    pub p8_key_path: PathBuf,
}

impl MacosConfig {
    pub fn load(brand: &str) -> Result<Self, String> {
        let path: PathBuf = ["configs", brand, "macos", "config.json"].iter().collect();
        let bytes = fs::read(&path).map_err(|e| format!("Failed to read {path:?}: {e}"))?;
        let config: Self =
            serde_json::from_slice(&bytes).map_err(|e| format!("Failed to parse config: {e}"))?;

        // Verify key exists
        if !config.p8_key_path.exists() {
            return Err(format!(
                "P8 key file not found at: {:?}",
                config.p8_key_path
            ));
        }

        Ok(config)
    }
}
