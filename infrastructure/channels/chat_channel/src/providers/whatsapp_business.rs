use serde::Deserialize;

/// WhatsApp Business API configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct WhatsAppBusinessConfig {
    pub phone_number_id: String,
    pub access_token: String,
    #[serde(default)]
    pub api_version: Option<String>,
}
