use serde::Deserialize;

/// Firebase Cloud Messaging configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct FcmConfig {
    pub service_account_key: String,
    #[serde(default)]
    pub project_id: Option<String>,
}
