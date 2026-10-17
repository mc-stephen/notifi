use serde::Deserialize;

/// Termii SMS configuration — focused on Nigeria/Africa.
#[derive(Debug, Clone, Deserialize)]
pub struct TermiiConfig {
    pub api_key: String,
    pub sender_id: String,
    #[serde(default)]
    pub base_url: Option<String>,
}
