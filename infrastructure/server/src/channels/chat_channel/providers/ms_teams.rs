use futures::future::BoxFuture;
use reqwest::Client;
use serde::Deserialize;

use super::ChatSender;
use crate::channels::chat_channel::ChatMessage;

/// Microsoft Teams configuration (incoming webhook).
#[derive(Debug, Clone, Deserialize)]
pub struct MsTeamsConfig {
    pub webhook_url: String,
    #[serde(default)]
    pub channel_name: Option<String>,
}

/// MS Teams chat provider controller — webhook delivery.
pub struct MsTeamsProvider {
    config: MsTeamsConfig,
    client: Client,
}

impl MsTeamsProvider {
    pub fn new(config: MsTeamsConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }
}

impl ChatSender for MsTeamsProvider {
    fn send(&self, msg: &ChatMessage) -> BoxFuture<'static, Result<(), String>> {
        let text = msg.text.clone();
        let url = self.config.webhook_url.clone();
        let client = self.client.clone();

        Box::pin(async move {
            let response = client
                .post(&url)
                .json(&serde_json::json!({ "text": text }))
                .send()
                .await
                .map_err(|e| format!("MS Teams webhook request failed: {e}"))?;
            if response.status().is_success() {
                Ok(())
            } else {
                let err = response.text().await.unwrap_or_default();
                Err(format!("MS Teams webhook error: {err}"))
            }
        })
    }
}

#[cfg(test)]
mod ms_teams_tests {
    use super::*;
    use crate::testing::http_stub;

    fn config(url: String) -> MsTeamsConfig {
        MsTeamsConfig {
            webhook_url: url,
            channel_name: Some("General".to_string()),
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
    fn config_parses_with_optional_channel_name() {
        let c: MsTeamsConfig =
            serde_json::from_str(r#"{"webhook_url":"https://example.com/webhook"}"#)
                .expect("channel_name stays optional");
        assert!(c.channel_name.is_none());
        let with_name: MsTeamsConfig = serde_json::from_str(
            r#"{"webhook_url":"https://example.com/webhook","channel_name":"News"}"#,
        )
        .expect("parses");
        assert_eq!(with_name.channel_name.as_deref(), Some("News"));
    }

    #[tokio::test]
    async fn webhook_posts_text_payload() {
        let stub = http_stub::start(200, "OK", "1").await;
        MsTeamsProvider::new(config(format!("http://127.0.0.1:{}", stub.port)))
            .send(&msg())
            .await
            .expect("teams webhook 200 delivers");
        let raw = stub.request().await.expect("request captured");
        assert!(raw.contains(r#"{"text":"hello"}"#), "payload: {raw}");
    }

    #[tokio::test]
    async fn surfaces_provider_error_body() {
        let stub = http_stub::start(400, "Bad Request", "Summary or TextCard is required.").await;
        let err = MsTeamsProvider::new(config(format!("http://127.0.0.1:{}", stub.port)))
            .send(&msg())
            .await
            .expect_err("400 is a failed Teams send");
        assert!(err.starts_with("MS Teams webhook error:"), "prefix: {err}");
        assert!(err.contains("TextCard is required"), "body surfaced: {err}");
    }

    #[tokio::test]
    #[ignore = "live MS Teams delivery — needs NOTIFI_MS_TEAMS_WEBHOOK_URL"]
    async fn live_ms_teams_delivery() {
        let url = std::env::var("NOTIFI_MS_TEAMS_WEBHOOK_URL")
            .expect("NOTIFI_MS_TEAMS_WEBHOOK_URL not set");
        MsTeamsProvider::new(MsTeamsConfig {
            webhook_url: url,
            channel_name: None,
        })
        .send(&ChatMessage {
            channel_id: None,
            text: format!(
                "Notifi live-check {}",
                chrono::Utc::now().timestamp_millis()
            ),
            metadata: None,
        })
        .await
        .expect("real MS Teams webhook accepted the message");
    }
}
