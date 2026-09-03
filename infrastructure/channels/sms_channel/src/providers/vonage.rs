use serde::Deserialize;

/// Vonage (formerly Nexmo) SMS configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct VonageConfig {
    pub api_key: String,
    pub api_secret: String,
    pub from_number: String,
}
