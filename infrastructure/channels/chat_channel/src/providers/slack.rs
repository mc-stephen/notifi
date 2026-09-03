use serde::Deserialize;

/// Slack configuration (webhook or bot API).
#[derive(Debug, Clone, Deserialize)]
pub struct SlackConfig {
    #[serde(flatten)]
    pub auth: SlackAuth,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum SlackAuth {
    #[serde(rename = "webhook")]
    Webhook { webhook_url: String },
    #[serde(rename = "bot")]
    BotApi { bot_token: String, channel_id: String },
}
