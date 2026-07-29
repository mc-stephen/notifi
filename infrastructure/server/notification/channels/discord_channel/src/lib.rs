mod config;

pub use config::DiscordConfig;

use reqwest::Client;
use serde::Serialize;

pub struct DiscordMessage {
    pub text: String,
}

#[derive(Serialize)]
struct DiscordPayload {
    content: String,
}

pub struct DiscordProvider {
    config: DiscordConfig,
    client: Client,
}

impl DiscordProvider {
    pub fn new(config: DiscordConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    pub async fn send_message(&self, msg: &DiscordMessage) -> Result<(), String> {
        let payload = DiscordPayload { content: msg.text.clone() };

        match &self.config {
            DiscordConfig::Webhook { webhook_url } => {
                let response = self.client.post(webhook_url)
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| format!("Webhook request failed: {e}"))?;

                if response.status().is_success() {
                    Ok(())
                } else {
                    let err = response.text().await.unwrap_or_default();
                    Err(format!("Webhook error: {err}"))
                }
            }
            DiscordConfig::BotApi { bot_token, channel_id } => {
                let url = format!("https://discord.com/api/v10/channels/{}/messages", channel_id);
                let response = self.client.post(&url)
                    .header("Authorization", format!("Bot {}", bot_token))
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| format!("Bot API request failed: {e}"))?;

                if response.status().is_success() {
                    Ok(())
                } else {
                    let err = response.text().await.unwrap_or_default();
                    Err(format!("Bot API error: {err}"))
                }
            }
        }
    }
}
