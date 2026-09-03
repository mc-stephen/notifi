use serde::Deserialize;

/// Microsoft Teams configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct MsTeamsConfig {
    pub webhook_url: String,
    #[serde(default)]
    pub channel_name: Option<String>,
}
