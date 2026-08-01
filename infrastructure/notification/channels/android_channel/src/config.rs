use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct AndroidConfig {
    pub service_account_path: PathBuf,
}

impl AndroidConfig {
    pub fn load(brand: &str) -> Result<Self, String> {
        // Based on the convention, service account is at configs/{brand}/fcm/service-account.json
        let path: PathBuf = ["configs", brand, "fcm", "service-account.json"]
            .iter()
            .collect();
        if !path.exists() {
            return Err(format!("Service account file not found at: {:?}", path));
        }
        Ok(AndroidConfig {
            service_account_path: path,
        })
    }
}
