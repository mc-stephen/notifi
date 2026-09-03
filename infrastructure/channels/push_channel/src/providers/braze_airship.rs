use serde::Deserialize;

/// Braze (formerly Airship) configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct BrazeAirshipConfig {
    pub api_key: String,
    pub app_id: String,
    pub base_url: String,
}
