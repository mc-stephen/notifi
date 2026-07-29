mod config;

pub use config::SlackConfig;

use reqwest::Client;
use serde::Serialize;

pub struct SlackMessage {
    pub text: String,
}

#[derive(Serialize)]
struct WebhookPayload {
    text: String,
}

#[derive(Serialize)]
struct BotPayload {
    channel: String,
    text: String,
}

pub struct SlackProvider {
    config: SlackConfig,
    client: Client,
}

impl SlackProvider {
    pub fn new(config: SlackConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    pub async fn send_message(&self, msg: &SlackMessage) -> Result<(), String> {
        match &self.config {
            SlackConfig::Webhook { webhook_url } => {
                let payload = WebhookPayload { text: msg.text.clone() };
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
            SlackConfig::BotApi { bot_token, channel_id } => {
                let payload = BotPayload {
                    channel: channel_id.clone(),
                    text: msg.text.clone(),
                };
                let response = self.client.post("https://slack.com/api/chat.postMessage")
                    .header("Authorization", format!("Bearer {}", bot_token))
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
