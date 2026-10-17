use serde::Deserialize;

/// Apple Push Notification Service configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ApnsConfig {
    pub key_id: String,
    pub team_id: String,
    pub bundle_id: String,
    pub private_key: String,
    #[serde(default = "default_sandbox")]
    pub sandbox: bool,
}

fn default_sandbox() -> bool {
    true
}
