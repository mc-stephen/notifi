use futures::future::BoxFuture;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::SmsSender;

/// Production API root — used when the channel config leaves `base_url`
/// unset; unit tests point it at a local stub instead.
pub const API_BASE: &str = "https://api.termii.com";

/// Termii SMS configuration — focused on Nigeria/Africa.
#[derive(Debug, Clone, Deserialize)]
pub struct TermiiConfig {
    pub api_key: String,
    pub sender_id: String,
    #[serde(default)]
    pub base_url: Option<String>,
}

/// Termii SMS provider implementation.
pub struct TermiiProvider {
    config: TermiiConfig,
    client: Client,
}

impl TermiiProvider {
    pub fn new(config: TermiiConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    fn api_base(&self) -> &str {
        self.config.base_url.as_deref().unwrap_or(API_BASE)
    }
}

#[derive(Serialize)]
struct TermiiSendMessage {
    to: String,
    from: String,
    sms: String,
    api_key: String,
    /// Termii's plain SMS route ("dndata" is their carrier-specific route).
    channel: &'static str,
}

impl SmsSender for TermiiProvider {
    fn send(&self, to: &str, text: &str) -> BoxFuture<'static, Result<(), String>> {
        let url = format!("{}/api/sms/send", self.api_base());
        let payload = TermiiSendMessage {
            to: to.to_string(),
            from: self.config.sender_id.clone(),
            sms: text.to_string(),
            api_key: self.config.api_key.clone(),
            channel: "generic",
        };
        let client = self.client.clone();

        Box::pin(async move {
            let response = client
                .post(url)
                .json(&payload)
                .send()
                .await
                .map_err(|e| format!("Termii request failed: {e}"))?;

            if response.status().is_success() {
                Ok(())
            } else {
                let err = response.text().await.unwrap_or_default();
                Err(format!("Termii error: {err}"))
            }
        })
    }
}

#[cfg(test)]
mod termii_tests {
    use super::*;
    use crate::channels::sms_channel::providers::SmsSender;
    use crate::testing::http_stub;

    const KEY: &str = "sk_live_test_key";
    const SENDER: &str = "Notifi";

    fn stub_config(port: u16) -> TermiiConfig {
        TermiiConfig {
            api_key: KEY.to_string(),
            sender_id: SENDER.to_string(),
            base_url: Some(format!("http://127.0.0.1:{port}")),
        }
    }

    #[test]
    fn config_defaults_base_url_when_absent() {
        let c: TermiiConfig =
            serde_json::from_str(r#"{"api_key":"k","sender_id":"s"}"#).expect("parses");
        let p = TermiiProvider::new(c);
        assert_eq!(p.api_base(), API_BASE);
    }

    #[tokio::test]
    async fn posts_json_to_sms_send_and_accepts_2xx() {
        let stub = http_stub::start(200, "OK", r#"{"code":"ok","message_id":"551"}"#).await;
        let provider = TermiiProvider::new(stub_config(stub.port));
        provider
            .send("+2348012345678", "welcome")
            .await
            .expect("200 is a delivered Termii send");
        let raw = stub.request().await.expect("request captured");
        assert!(raw.contains("POST /api/sms/send"), "send endpoint: {raw}");
        assert!(raw.contains(r#""to":"+2348012345678""#), "to field: {raw}");
        assert!(
            raw.contains(r#""from":"Notifi""#),
            "from is the sender id: {raw}"
        );
        assert!(raw.contains(r#""sms":"welcome""#), "message field: {raw}");
        assert!(
            raw.contains(r#""api_key":"sk_live_test_key""#),
            "api key rides the body (Termii contract): {raw}"
        );
        assert!(
            raw.contains(r#""channel":"generic""#),
            "plain SMS route: {raw}"
        );
    }

    #[tokio::test]
    async fn surfaces_provider_error_body() {
        let stub = http_stub::start(
            400,
            "Bad Request",
            r#"{"message":"Invalid API key or recipient"}"#,
        )
        .await;
        let err = TermiiProvider::new(stub_config(stub.port))
            .send("+2348012345678", "hi")
            .await
            .expect_err("400 is a failed Termii send");
        assert!(err.starts_with("Termii error:"), "prefix: {err}");
        assert!(
            err.contains("Invalid API key or recipient"),
            "API error body surfaced: {err}"
        );
    }

    #[tokio::test]
    #[ignore = "live Termii delivery — needs NOTIFI_TERMII_API_KEY, \
                NOTIFI_TERMII_SENDER_ID (registered sender id), NOTIFI_TERMII_TO"]
    async fn live_termii_delivery() {
        let config = TermiiConfig {
            api_key: std::env::var("NOTIFI_TERMII_API_KEY").expect("NOTIFI_TERMII_API_KEY not set"),
            sender_id: std::env::var("NOTIFI_TERMII_SENDER_ID")
                .expect("NOTIFI_TERMII_SENDER_ID not set"),
            base_url: None,
        };
        let to = std::env::var("NOTIFI_TERMII_TO").expect("NOTIFI_TERMII_TO not set");
        let provider = TermiiProvider::new(config);
        let body = format!(
            "Notifi live-check {}",
            chrono::Utc::now().timestamp_millis()
        );
        provider
            .send(&to, &body)
            .await
            .expect("real Termii API accepted the message");
    }
}
