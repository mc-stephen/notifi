mod config;

pub use config::TelegramConfig;

use reqwest::Client;
use serde::Serialize;

pub struct TelegramMessage {
    pub chat_id: String,
    pub text: String,
}

#[derive(Serialize)]
struct TelegramPayload {
    chat_id: String,
    text: String,
}

pub struct TelegramProvider {
    config: TelegramConfig,
    client: Client,
}

impl TelegramProvider {
    pub fn new(config: TelegramConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    pub async fn send_message(&self, msg: &TelegramMessage) -> Result<(), String> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.config.bot_token
        );

        let payload = TelegramPayload {
            chat_id: msg.chat_id.clone(),
            text: msg.text.clone(),
        };

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Failed to send request to Telegram: {e}"))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(format!("Telegram API error: {error_text}"))
        }
    }
}
