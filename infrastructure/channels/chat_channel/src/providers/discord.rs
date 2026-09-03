use serde::Deserialize;

/// Discord configuration (webhook or bot API).
#[derive(Debug, Clone, Deserialize)]
pub struct DiscordConfig {
    #[serde(flatten)]
    pub auth: DiscordAuth,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum DiscordAuth {
    #[serde(rename = "webhook")]
    Webhook { webhook_url: String },
    #[serde(rename = "bot")]
    BotApi { bot_token: String, channel_id: String },
}
