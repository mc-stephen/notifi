use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Default)]
pub struct LinuxConfig {
    // Generic channel, config may be empty for now or store global settings
}

impl LinuxConfig {
    pub fn load(brand: &str) -> Result<Self, String> {
        let path: PathBuf = ["configs", brand, "linux", "config.json"].iter().collect();
        // Allow missing config file if channel is generic
        if !path.exists() {
            return Ok(LinuxConfig::default());
        }
        let bytes = fs::read(&path).map_err(|e| format!("Failed to read {path:?}: {e}"))?;
        serde_json::from_slice::<Self>(&bytes).map_err(|e| format!("Failed to parse config: {e}"))
    }
}
