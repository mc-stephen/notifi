use futures::future::BoxFuture;
use reqwest::Client;
use serde::Deserialize;

use super::ChatSender;
use crate::channels::chat_channel::ChatMessage;

/// Production Bot API root — unit tests point a local stub at it instead
/// (webhook URLs carry their own endpoint, so they need no seam).
#[cfg(test)]
pub(crate) const DISCORD_API_BASE: &str = "https://discord.com";

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
    BotApi {
        bot_token: String,
        channel_id: String,
    },
}

/// Discord chat provider controller — webhook or Bot API per auth config.
pub struct DiscordProvider {
    auth: DiscordAuth,
    client: Client,
    /// Bot API root override for tests; production never sets it.
    #[cfg(test)]
    api_base: Option<String>,
}

impl DiscordProvider {
    pub fn new(config: DiscordConfig) -> Self {
        Self {
            auth: config.auth,
            client: Client::new(),
            #[cfg(test)]
            api_base: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_with_api_base(config: DiscordConfig, api_base: String) -> Self {
        Self {
            #[cfg(test)]
            api_base: Some(api_base),
            ..Self::new(config)
        }
    }
}

impl ChatSender for DiscordProvider {
    fn send(&self, msg: &ChatMessage) -> BoxFuture<'static, Result<(), String>> {
        let text = msg.text.clone();
        let client = self.client.clone();
        match self.auth.clone() {
            DiscordAuth::Webhook { webhook_url } => Box::pin(async move {
                let response = client
                    .post(&webhook_url)
                    .json(&serde_json::json!({ "content": text }))
                    .send()
                    .await
                    .map_err(|e| format!("Discord webhook request failed: {e}"))?;
                if response.status().is_success() {
                    Ok(())
                } else {
                    let err = response.text().await.unwrap_or_default();
                    Err(format!("Discord webhook error: {err}"))
                }
            }),
            DiscordAuth::BotApi {
                bot_token,
                channel_id,
            } => {
                #[cfg(test)]
                let url = match &self.api_base {
                    Some(base) => format!("{base}/api/v10/channels/{channel_id}/messages"),
                    None => format!("{DISCORD_API_BASE}/api/v10/channels/{channel_id}/messages"),
                };
                #[cfg(not(test))]
                let url = format!("https://discord.com/api/v10/channels/{channel_id}/messages");
                let token = bot_token;
                Box::pin(async move {
                    let response = client
                        .post(&url)
                        .header("Authorization", format!("Bot {token}"))
                        .json(&serde_json::json!({ "content": text }))
                        .send()
                        .await
                        .map_err(|e| format!("Discord bot API request failed: {e}"))?;
                    if response.status().is_success() {
                        Ok(())
                    } else {
                        let err = response.text().await.unwrap_or_default();
                        Err(format!("Discord bot API error: {err}"))
                    }
                })
            }
        }
    }
}

#[cfg(test)]
mod discord_tests {
    use super::*;
    use crate::testing::http_stub;

    const TOKEN: &str = "dc-test-bot-token";

    fn bot_config() -> DiscordConfig {
        DiscordConfig {
            auth: DiscordAuth::BotApi {
                bot_token: TOKEN.to_string(),
                channel_id: "CH789".to_string(),
            },
        }
    }

    fn msg() -> ChatMessage {
        ChatMessage {
            channel_id: None,
            text: "hello".to_string(),
            metadata: None,
        }
    }

    #[test]
    fn config_parses_both_auth_tagged_variants() {
        let webhook: DiscordConfig =
            serde_json::from_str(r#"{"type":"webhook","webhook_url":"https://example.com/hook"}"#)
                .expect("webhook variant parses (tagged)");
        assert!(matches!(webhook.auth, DiscordAuth::Webhook { .. }));
        let bot: DiscordConfig =
            serde_json::from_str(r#"{"type":"bot","bot_token":"t","channel_id":"C1"}"#)
                .expect("bot variant parses (tagged)");
        assert!(matches!(bot.auth, DiscordAuth::BotApi { .. }));
    }

    #[tokio::test]
    async fn webhook_posts_content_payload() {
        let stub = http_stub::start(200, "OK", "").await;
        DiscordProvider::new(DiscordConfig {
            auth: DiscordAuth::Webhook {
                webhook_url: format!("http://127.0.0.1:{}", stub.port),
            },
        })
        .send(&msg())
        .await
        .expect("discord webhook 200 delivers");
        let raw = stub.request().await.expect("request captured");
        assert!(raw.contains(r#"{"content":"hello"}"#), "payload: {raw}");
    }

    #[tokio::test]
    async fn bot_api_posts_with_bot_token_header() {
        let stub = http_stub::start(200, "OK", r#"{"id":"m1"}"#).await;
        DiscordProvider::new_with_api_base(bot_config(), format!("http://127.0.0.1:{}", stub.port))
            .send(&msg())
            .await
            .expect("discord bot 200 delivers");
        let raw = stub.request().await.expect("request captured");
        assert!(
            raw.contains("POST /api/v10/channels/CH789/messages"),
            "channel path: {raw}"
        );
        assert!(
            raw.to_ascii_lowercase().contains("authorization: bot "),
            "bot token header: {raw}"
        );
        assert!(raw.contains(r#""content":"hello""#), "content body: {raw}");
    }

    #[tokio::test]
    async fn bot_api_surfaces_error_body() {
        let stub = http_stub::start(
            403,
            "Forbidden",
            r#"{"code":50001,"message":"Missing Access"}"#,
        )
        .await;
        let err = DiscordProvider::new_with_api_base(
            bot_config(),
            format!("http://127.0.0.1:{}", stub.port),
        )
        .send(&msg())
        .await
        .expect_err("403 is a failed Discord send");
        assert!(err.starts_with("Discord bot API error:"), "prefix: {err}");
        assert!(err.contains("Missing Access"), "body surfaced: {err}");
    }

    #[tokio::test]
    #[ignore = "live Discord delivery — needs NOTIFI_DISCORD_BOT_TOKEN, NOTIFI_DISCORD_CHANNEL_ID"]
    async fn live_discord_bot_delivery() {
        let config = DiscordConfig {
            auth: DiscordAuth::BotApi {
                bot_token: std::env::var("NOTIFI_DISCORD_BOT_TOKEN")
                    .expect("NOTIFI_DISCORD_BOT_TOKEN not set"),
                channel_id: std::env::var("NOTIFI_DISCORD_CHANNEL_ID")
                    .expect("NOTIFI_DISCORD_CHANNEL_ID not set"),
            },
        };
        DiscordProvider::new(config)
            .send(&ChatMessage {
                channel_id: None,
                text: format!(
                    "Notifi live-check {}",
                    chrono::Utc::now().timestamp_millis()
                ),
                metadata: None,
            })
            .await
            .expect("real Discord API accepted the message");
    }
}
