use serde::Deserialize;

/// Telegram Bot API configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub chat_id: String,
}
