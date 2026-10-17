use serde::Deserialize;

/// EbulkSms configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct EbulkSmsConfig {
    pub api_key: String,
    pub sender_id: String,
    #[serde(default)]
    pub base_url: Option<String>,
}
