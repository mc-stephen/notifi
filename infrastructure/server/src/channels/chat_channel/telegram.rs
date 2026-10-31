use futures::future::BoxFuture;
use reqwest::Client;
use serde::Deserialize;

use super::ChatSender;
use crate::channels::chat_channel::ChatMessage;

/// Production Bot API root — unit tests point a local stub at it instead.
#[cfg(test)]
pub(crate) const TELEGRAM_API_BASE: &str = "https://api.telegram.org";

/// Telegram Bot API configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub chat_id: String,
}

/// Telegram chat provider controller.
pub struct TelegramProvider {
    config: TelegramConfig,
    client: Client,
    /// Bot API root override for tests; production never sets it.
    #[cfg(test)]
    api_base: Option<String>,
}

impl TelegramProvider {
    pub fn new(config: TelegramConfig) -> Self {
        Self {
            config,
            client: Client::new(),
            #[cfg(test)]
            api_base: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_with_api_base(config: TelegramConfig, api_base: String) -> Self {
        Self {
            #[cfg(test)]
            api_base: Some(api_base),
            ..Self::new(config)
        }
    }

    /// Credential check for the provider tester: calls getMe — an
    /// authenticated read of the bot's identity that sends nothing.
    pub async fn verify(&self) -> Result<(), String> {
        let url = {
            #[cfg(test)]
            {
                match &self.api_base {
                    Some(base) => format!("{base}/bot{}/getMe", self.config.bot_token),
                    None => format!("{TELEGRAM_API_BASE}/bot{}/getMe", self.config.bot_token),
                }
            }
            #[cfg(not(test))]
            {
                format!(
                    "https://api.telegram.org/bot{}/getMe",
                    self.config.bot_token
                )
            }
        };
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Telegram request failed: {e}"))?;
        let status = response.status();
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Telegram response unreadable (HTTP {status}): {e}"))?;
        if status.is_success() && body.get("ok") == Some(&serde_json::Value::Bool(true)) {
            Ok(())
        } else {
            let reason = body
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Err(format!("Telegram rejected the credential: {reason}"))
        }
    }
}

impl ChatSender for TelegramProvider {
    fn send(&self, msg: &ChatMessage) -> BoxFuture<'static, Result<(), String>> {
        // The title is the headline — plain text only (no parse_mode is
        // set, so user escape characters stay literal); the body follows
        // beneath.
        let text = match msg.title.as_deref() {
            Some(title) if !title.is_empty() => format!("{title}\n{}", msg.text),
            _ => msg.text.clone(),
        };
        let client = self.client.clone();
        let url = {
            #[cfg(test)]
            {
                match &self.api_base {
                    Some(base) => format!("{base}/bot{}/sendMessage", self.config.bot_token),
                    None => format!(
                        "{TELEGRAM_API_BASE}/bot{}/sendMessage",
                        self.config.bot_token
                    ),
                }
            }
            #[cfg(not(test))]
            {
                format!(
                    "https://api.telegram.org/bot{}/sendMessage",
                    self.config.bot_token
                )
            }
        };
        let chat_id = self.config.chat_id.clone();
        Box::pin(async move {
            let response = client
                .post(&url)
                .json(&serde_json::json!({ "chat_id": chat_id, "text": text }))
                .send()
                .await
                .map_err(|e| format!("Telegram request failed: {e}"))?;
            if response.status().is_success() {
                Ok(())
            } else {
                let err = response.text().await.unwrap_or_default();
                Err(format!("Telegram API error: {err}"))
            }
        })
    }
}

#[cfg(test)]
mod telegram_tests {
    use super::*;
    use crate::testing::http_stub;

    const TOKEN: &str = "tg-test-token";

    fn config() -> TelegramConfig {
        TelegramConfig {
            bot_token: TOKEN.to_string(),
            chat_id: "CHAT456".to_string(),
        }
    }

    fn stub_provider(port: u16) -> TelegramProvider {
        TelegramProvider::new_with_api_base(config(), format!("http://127.0.0.1:{port}"))
    }

    fn msg() -> ChatMessage {
        ChatMessage {
            channel_id: None,
            title: None,
            text: "hello".to_string(),
            metadata: None,
        }
    }

    #[test]
    fn config_parses_plain_fields() {
        let c: TelegramConfig =
            serde_json::from_str(r#"{"bot_token":"t","chat_id":"c"}"#).expect("parses");
        assert_eq!(c.bot_token, "t");
        assert_eq!(c.chat_id, "c");
    }

    #[tokio::test]
    async fn posts_to_bot_sendmessage() {
        let stub = http_stub::start(200, "OK", r#"{"ok":true}"#).await;
        stub_provider(stub.port)
            .send(&msg())
            .await
            .expect("telegram 200 delivers");
        let raw = stub.request().await.expect("request captured");
        assert!(
            raw.contains(&format!("POST /bot{TOKEN}/sendMessage")),
            "bot token in path: {raw}"
        );
        assert!(raw.contains(r#""chat_id":"CHAT456""#), "chat_id: {raw}");
        assert!(raw.contains(r#""text":"hello""#), "text: {raw}");
    }

    #[tokio::test]
    async fn surfaces_provider_error_body() {
        let stub = http_stub::start(
            400,
            "Bad Request",
            r#"{"ok":false,"description":"chat not found"}"#,
        )
        .await;
        let err = stub_provider(stub.port)
            .send(&msg())
            .await
            .expect_err("400 is a failed Telegram send");
        assert!(err.starts_with("Telegram API error:"), "prefix: {err}");
        assert!(err.contains("chat not found"), "body surfaced: {err}");
    }

    #[tokio::test]
    async fn title_arrives_as_the_plain_first_line() {
        // No parse_mode is set (user escape characters stay literal), so
        // the title is a plain first line.
        let mut titled = msg();
        titled.title = Some("Deploy done".to_string());
        let stub = http_stub::start(200, "OK", r#"{"ok":true}"#).await;
        stub_provider(stub.port)
            .send(&titled)
            .await
            .expect("titled delivery");
        let raw = stub.request().await.expect("request captured");
        assert!(
            raw.contains(r#""text":"Deploy done\nhello""#),
            "headline: {raw}"
        );
    }

    #[tokio::test]
    async fn verify_reads_the_bot_identity() {
        let stub = http_stub::start(200, "OK", r#"{"ok":true,"result":{"id":1}}"#).await;
        stub_provider(stub.port)
            .verify()
            .await
            .expect("ok:true getMe proves the bot token");
        let raw = stub.request().await.expect("request captured");
        assert!(
            raw.contains(&format!("GET /bot{TOKEN}/getMe")),
            "getMe path: {raw}"
        );
    }

    #[tokio::test]
    async fn verify_rejects_a_bad_bot_token() {
        let stub = http_stub::start(
            401,
            "Unauthorized",
            r#"{"ok":false,"description":"Unauthorized"}"#,
        )
        .await;
        let err = stub_provider(stub.port)
            .verify()
            .await
            .expect_err("a rejected bot token fails the probe");
        assert!(err.contains("Telegram rejected the credential"), "{err}");
    }

    #[tokio::test]
    #[ignore = "live Telegram delivery — needs NOTIFI_TELEGRAM_BOT_TOKEN, NOTIFI_TELEGRAM_CHAT_ID"]
    async fn live_telegram_delivery() {
        let config = TelegramConfig {
            bot_token: std::env::var("NOTIFI_TELEGRAM_BOT_TOKEN")
                .expect("NOTIFI_TELEGRAM_BOT_TOKEN not set"),
            chat_id: std::env::var("NOTIFI_TELEGRAM_CHAT_ID")
                .expect("NOTIFI_TELEGRAM_CHAT_ID not set"),
        };
        TelegramProvider::new(config)
            .send(&ChatMessage {
                channel_id: None,
                title: None,
                text: format!(
                    "Notifi live-check {}",
                    chrono::Utc::now().timestamp_millis()
                ),
                metadata: None,
            })
            .await
            .expect("real Telegram API accepted the message");
    }
}
