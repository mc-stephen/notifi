use futures::future::BoxFuture;
use reqwest::Client;
use serde::Deserialize;

use super::ChatSender;
use crate::channels::chat_channel::ChatMessage;

/// Production Bot API root — unit tests point a local stub at it instead
/// (webhook URLs carry their own endpoint, so they need no seam).
#[cfg(test)]
pub(crate) const SLACK_API_BASE: &str = "https://slack.com";

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
    BotApi {
        bot_token: String,
        channel_id: String,
    },
}

/// Slack chat provider controller — webhook or Bot API per auth config.
pub struct SlackProvider {
    auth: SlackAuth,
    client: Client,
    /// Bot API root override for tests; production never sets it.
    #[cfg(test)]
    api_base: Option<String>,
}

impl SlackProvider {
    pub fn new(config: SlackConfig) -> Self {
        Self {
            auth: config.auth,
            client: Client::new(),
            #[cfg(test)]
            api_base: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_with_api_base(config: SlackConfig, api_base: String) -> Self {
        Self {
            #[cfg(test)]
            api_base: Some(api_base),
            ..Self::new(config)
        }
    }
}

impl ChatSender for SlackProvider {
    fn send(&self, msg: &ChatMessage) -> BoxFuture<'static, Result<(), String>> {
        let text = msg.text.clone();
        let client = self.client.clone();
        match self.auth.clone() {
            SlackAuth::Webhook { webhook_url } => Box::pin(async move {
                let response = client
                    .post(&webhook_url)
                    .json(&serde_json::json!({ "text": text }))
                    .send()
                    .await
                    .map_err(|e| format!("Slack webhook request failed: {e}"))?;
                if response.status().is_success() {
                    Ok(())
                } else {
                    let err = response.text().await.unwrap_or_default();
                    Err(format!("Slack webhook error: {err}"))
                }
            }),
            SlackAuth::BotApi {
                bot_token,
                channel_id,
            } => {
                #[cfg(test)]
                let url = match &self.api_base {
                    Some(base) => format!("{base}/api/chat.postMessage"),
                    None => format!("{SLACK_API_BASE}/api/chat.postMessage"),
                };
                #[cfg(not(test))]
                let url = "https://slack.com/api/chat.postMessage".to_string();
                let token = bot_token;
                Box::pin(async move {
                    let response = client
                        .post(&url)
                        .header("Authorization", format!("Bearer {token}"))
                        .json(&serde_json::json!({ "channel": channel_id, "text": text }))
                        .send()
                        .await
                        .map_err(|e| format!("Slack bot API request failed: {e}"))?;
                    if !response.status().is_success() {
                        let err = response.text().await.unwrap_or_default();
                        return Err(format!("Slack bot API error: {err}"));
                    }
                    // Slack quirk: HTTP 200 with {"ok":false} is a logical
                    // failure — the ok flag has the final say.
                    let body = response.text().await.unwrap_or_default();
                    let slack_ok_false = serde_json::from_str::<serde_json::Value>(&body)
                        .is_ok_and(|parsed| {
                            parsed.get("ok") == Some(&serde_json::Value::Bool(false))
                        });
                    if slack_ok_false {
                        return Err(format!("Slack bot API error: {body}"));
                    }
                    Ok(())
                })
            }
        }
    }
}

#[cfg(test)]
mod slack_tests {
    use super::*;
    use crate::testing::http_stub;

    const TOKEN: &str = "xoxb-test-token";

    fn bot_config() -> SlackConfig {
        SlackConfig {
            auth: SlackAuth::BotApi {
                bot_token: TOKEN.to_string(),
                channel_id: "C123".to_string(),
            },
        }
    }

    fn stub_bot_provider(port: u16) -> SlackProvider {
        SlackProvider::new_with_api_base(bot_config(), format!("http://127.0.0.1:{port}"))
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
        let webhook: SlackConfig =
            serde_json::from_str(r#"{"type":"webhook","webhook_url":"https://example.com/hook"}"#)
                .expect("webhook variant parses (tagged)");
        assert!(matches!(webhook.auth, SlackAuth::Webhook { .. }));
        let bot: SlackConfig =
            serde_json::from_str(r#"{"type":"bot","bot_token":"t","channel_id":"C1"}"#)
                .expect("bot variant parses (tagged)");
        assert!(matches!(bot.auth, SlackAuth::BotApi { .. }));
    }

    #[tokio::test]
    async fn webhook_posts_text_payload() {
        let stub = http_stub::start(200, "OK", "ok").await;
        SlackProvider::new(SlackConfig {
            auth: SlackAuth::Webhook {
                webhook_url: format!("http://127.0.0.1:{}", stub.port),
            },
        })
        .send(&msg())
        .await
        .expect("slack webhook 200 delivers");
        let raw = stub.request().await.expect("request captured");
        assert!(raw.contains(r#"{"text":"hello"}"#), "payload: {raw}");
    }

    #[tokio::test]
    async fn bot_api_posts_to_chat_with_bearer() {
        let stub = http_stub::start(200, "OK", r#"{"ok":true}"#).await;
        stub_bot_provider(stub.port)
            .send(&msg())
            .await
            .expect("slack bot ok:true delivers");
        let raw = stub.request().await.expect("request captured");
        assert!(raw.contains("POST /api/chat.postMessage"), "url: {raw}");
        assert!(
            raw.to_ascii_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}")),
            "bearer header: {raw}"
        );
        assert!(raw.contains(r#""channel":"C123""#), "channel body: {raw}");
    }

    #[tokio::test]
    async fn bot_api_http_error_surfaces_body() {
        let stub = http_stub::start(400, "Bad Request", "invalid_auth").await;
        let err = stub_bot_provider(stub.port)
            .send(&msg())
            .await
            .expect_err("HTTP 400 is a failed send");
        assert!(err.starts_with("Slack bot API error:"), "prefix: {err}");
        assert!(err.contains("invalid_auth"), "body surfaced: {err}");
    }

    #[tokio::test]
    async fn bot_api_ok_false_is_a_failure() {
        // Slack quirk: HTTP 200 but {"ok":false} — must not count as sent.
        let stub = http_stub::start(200, "OK", r#"{"ok":false,"error":"channel_not_found"}"#).await;
        let err = stub_bot_provider(stub.port)
            .send(&msg())
            .await
            .expect_err("ok:false is a failed send");
        assert!(err.contains("channel_not_found"), "body surfaced: {err}");
    }

    #[tokio::test]
    #[ignore = "live Slack Bot API delivery — needs NOTIFI_SLACK_BOT_TOKEN, NOTIFI_SLACK_CHANNEL_ID"]
    async fn live_slack_bot_delivery() {
        let config = SlackConfig {
            auth: SlackAuth::BotApi {
                bot_token: std::env::var("NOTIFI_SLACK_BOT_TOKEN")
                    .expect("NOTIFI_SLACK_BOT_TOKEN not set"),
                channel_id: std::env::var("NOTIFI_SLACK_CHANNEL_ID")
                    .expect("NOTIFI_SLACK_CHANNEL_ID not set"),
            },
        };
        SlackProvider::new(config)
            .send(&ChatMessage {
                channel_id: None,
                text: format!(
                    "Notifi live-check {}",
                    chrono::Utc::now().timestamp_millis()
                ),
                metadata: None,
            })
            .await
            .expect("real Slack API accepted the message");
    }
}
