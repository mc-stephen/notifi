pub mod config;
pub mod providers;

pub use config::ChatConfig;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChatMessage {
    pub channel_id: Option<String>,
    pub text: String,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

pub struct ChatSender {
    config: config::ChatConfig,
    client: reqwest::Client,
}

impl ChatSender {
    pub fn new(config: config::ChatConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    pub async fn send_message(&self, msg: &ChatMessage) -> Result<(), String> {
        let provider = self.config.clone().to_provider();
        match provider {
            providers::ChatProvider::Slack(c) => {
                match c.auth {
                    providers::slack::SlackAuth::Webhook { webhook_url } => {
                        let payload = serde_json::json!({ "text": msg.text });
                        let response = self
                            .client
                            .post(&webhook_url)
                            .json(&payload)
                            .send()
                            .await
                            .map_err(|e| format!("Slack webhook request failed: {e}"))?;

                        if response.status().is_success() {
                            Ok(())
                        } else {
                            let err = response.text().await.unwrap_or_default();
                            Err(format!("Slack webhook error: {err}"))
                        }
                    }
                    providers::slack::SlackAuth::BotApi { bot_token, channel_id } => {
                        let payload = serde_json::json!({
                            "channel": channel_id,
                            "text": msg.text
                        });
                        let response = self
                            .client
                            .post("https://slack.com/api/chat.postMessage")
                            .header("Authorization", format!("Bearer {}", bot_token))
                            .json(&payload)
                            .send()
                            .await
                            .map_err(|e| format!("Slack bot API request failed: {e}"))?;

                        if response.status().is_success() {
                            Ok(())
                        } else {
                            let err = response.text().await.unwrap_or_default();
                            Err(format!("Slack bot API error: {err}"))
                        }
                    }
                }
            }
            providers::ChatProvider::Telegram(c) => {
                let url = format!(
                    "https://api.telegram.org/bot{}/sendMessage",
                    c.bot_token
                );
                let payload = serde_json::json!({
                    "chat_id": c.chat_id,
                    "text": msg.text
                });
                let response = self
                    .client
                    .post(&url)
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| format!("Telegram request failed: {e}"))?;

                if response.status().is_success() {
                    Ok(())
                } else {
                    let err = response.text().await.unwrap_or_default();
                    Err(format!("Telegram API error: {err}"))
                }
            }
            providers::ChatProvider::WhatsAppBusiness(c) => {
                let version = c.api_version.as_deref().unwrap_or("v18.0");
                let url = format!(
                    "https://graph.facebook.com/{}/{}//messages",
                    version, c.phone_number_id
                );
                let payload = serde_json::json!({
                    "messaging_product": "whatsapp",
                    "to": msg.channel_id.as_deref().unwrap_or(""),
                    "type": "text",
                    "text": { "body": msg.text }
                });
                let response = self
                    .client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", c.access_token))
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| format!("WhatsApp API request failed: {e}"))?;

                if response.status().is_success() {
                    Ok(())
                } else {
                    let err = response.text().await.unwrap_or_default();
                    Err(format!("WhatsApp API error: {err}"))
                }
            }
            providers::ChatProvider::Discord(c) => {
                match c.auth {
                    providers::discord::DiscordAuth::Webhook { webhook_url } => {
                        let payload = serde_json::json!({ "content": msg.text });
                        let response = self
                            .client
                            .post(&webhook_url)
                            .json(&payload)
                            .send()
                            .await
                            .map_err(|e| format!("Discord webhook request failed: {e}"))?;

                        if response.status().is_success() {
                            Ok(())
                        } else {
                            let err = response.text().await.unwrap_or_default();
                            Err(format!("Discord webhook error: {err}"))
                        }
                    }
                    providers::discord::DiscordAuth::BotApi { bot_token, channel_id } => {
                        let url = format!(
                            "https://discord.com/api/v10/channels/{}/messages",
                            channel_id
                        );
                        let payload = serde_json::json!({ "content": msg.text });
                        let response = self
                            .client
                            .post(&url)
                            .header("Authorization", format!("Bot {}", bot_token))
                            .json(&payload)
                            .send()
                            .await
                            .map_err(|e| format!("Discord bot API request failed: {e}"))?;

                        if response.status().is_success() {
                            Ok(())
                        } else {
                            let err = response.text().await.unwrap_or_default();
                            Err(format!("Discord bot API error: {err}"))
                        }
                    }
                }
            }
            providers::ChatProvider::MsTeams(c) => {
                let payload = serde_json::json!({ "text": msg.text });
                let response = self
                    .client
                    .post(&c.webhook_url)
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| format!("MS Teams webhook request failed: {e}"))?;

                if response.status().is_success() {
                    Ok(())
                } else {
                    let err = response.text().await.unwrap_or_default();
                    Err(format!("MS Teams webhook error: {err}"))
                }
            }
        }
    }
}
