use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(tag = "mode")]
pub enum WebhookConfig {
    #[serde(rename = "relay")]
    Relay {
        target_url: String,
        method: String, // e.g., "POST"
        headers: HashMap<String, String>,
        payload_template: String,
    },
    #[serde(rename = "function")]
    Function { function_id: String },
}

impl WebhookConfig {}
