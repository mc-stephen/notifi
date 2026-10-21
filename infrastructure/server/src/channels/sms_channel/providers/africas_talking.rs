use futures::future::BoxFuture;
use reqwest::Client;
use serde::Deserialize;

use super::SmsSender;

/// Production API root — used when the channel config leaves `base_url`
/// unset; unit tests point it at a local stub instead.
pub const API_BASE: &str = "https://api.africastalking.com";

/// Africa's Talking SMS configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct AfricasTalkingConfig {
    pub api_key: String,
    pub username: String,
    pub sender_id: String,
    #[serde(default)]
    pub base_url: Option<String>,
}

/// Africa's Talking SMS provider implementation.
pub struct AfricasTalkingProvider {
    config: AfricasTalkingConfig,
    client: Client,
}

impl AfricasTalkingProvider {
    pub fn new(config: AfricasTalkingConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    fn api_base(&self) -> &str {
        self.config.base_url.as_deref().unwrap_or(API_BASE)
    }
}

impl SmsSender for AfricasTalkingProvider {
    fn send(&self, to: &str, text: &str) -> BoxFuture<'static, Result<(), String>> {
        let url = format!("{}/version1/messaging", self.api_base());
        let mut params: Vec<(&str, String)> = vec![
            ("username", self.config.username.clone()),
            ("to", to.to_string()),
            ("message", text.to_string()),
        ];
        // The sender id is optional upstream — only send it when the
        // config actually names one.
        if !self.config.sender_id.is_empty() {
            params.push(("from", self.config.sender_id.clone()));
        }
        let api_key = self.config.api_key.clone();
        let client = self.client.clone();

        Box::pin(async move {
            let response = client
                .post(url)
                .header("ApiKey", api_key)
                .header("Accept", "application/json")
                .form(&params)
                .send()
                .await
                .map_err(|e| format!("Africa's Talking request failed: {e}"))?;

            if response.status().is_success() {
                Ok(())
            } else {
                let err = response.text().await.unwrap_or_default();
                Err(format!("Africa's Talking error: {err}"))
            }
        })
    }
}

#[cfg(test)]
mod africas_talking_tests {
    use super::*;
    use crate::channels::sms_channel::providers::SmsSender;
    use crate::testing::http_stub;

    const KEY: &str = "at_live_test_key";
    const USERNAME: &str = "notifi-sandbox";
    const SENDER: &str = "Notifi";

    fn stub_config(port: u16) -> AfricasTalkingConfig {
        AfricasTalkingConfig {
            api_key: KEY.to_string(),
            username: USERNAME.to_string(),
            sender_id: SENDER.to_string(),
            base_url: Some(format!("http://127.0.0.1:{port}")),
        }
    }

    #[test]
    fn config_parses_and_defaults_base_url() {
        let c: AfricasTalkingConfig =
            serde_json::from_str(r#"{"api_key":"k","username":"u","sender_id":"s"}"#)
                .expect("parses");
        let p = AfricasTalkingProvider::new(c);
        assert_eq!(p.api_base(), API_BASE);
        assert_eq!(p.config.sender_id, "s");
    }

    #[test]
    fn empty_sender_id_is_allowed() {
        // Sender id is optional upstream; empty config values must not
        // poison deserialization.
        let c: AfricasTalkingConfig =
            serde_json::from_str(r#"{"api_key":"k","username":"u","sender_id":""}"#)
                .expect("parses");
        let p = AfricasTalkingProvider::new(c);
        assert_eq!(p.config.sender_id, "");
    }

    #[tokio::test]
    async fn posts_form_with_apikey_header_and_accepts_2xx() {
        let stub = http_stub::start(
            201,
            "Created",
            r#"{"SMSMessageData":{"Recipients":[{"status":"Success"}]}}"#,
        )
        .await;
        AfricasTalkingProvider::new(stub_config(stub.port))
            .send("+254711123456", "welcome")
            .await
            .expect("201 is a delivered Africa's Talking send");
        let raw = stub.request().await.expect("request captured");
        assert!(
            raw.contains("POST /version1/messaging"),
            "messaging endpoint: {raw}"
        );
        let lowered = raw.to_ascii_lowercase();
        assert!(
            lowered.contains("apikey: at_live_test_key"),
            "ApiKey header: {raw}"
        );
        assert!(
            lowered.contains("accept: application/json"),
            "accept header: {raw}"
        );
        assert!(
            lowered.contains("content-type: application/x-www-form-urlencoded"),
            "form content type: {raw}"
        );
        assert!(
            raw.contains("username=notifi-sandbox"),
            "username field: {raw}"
        );
        // + previews as %2B in form encoding; AT field names lowercase.
        assert!(raw.contains("to=%2B254711123456"), "to field: {raw}");
        assert!(raw.contains("message=welcome"), "message field: {raw}");
        assert!(raw.contains("from=Notifi"), "sender id as from: {raw}");
    }

    #[tokio::test]
    async fn surfaces_provider_error_body() {
        let stub = http_stub::start(
            400,
            "Bad Request",
            r#"{"status":"error","errorMessage":"Invalid ApiKey"}"#,
        )
        .await;
        let err = AfricasTalkingProvider::new(stub_config(stub.port))
            .send("+254711123456", "hi")
            .await
            .expect_err("400 is a failed Africa's Talking send");
        assert!(err.starts_with("Africa's Talking error:"), "prefix: {err}");
        assert!(
            err.contains("Invalid ApiKey"),
            "API error body surfaced: {err}"
        );
    }

    #[tokio::test]
    async fn empty_sender_id_omits_from_field() {
        let stub =
            http_stub::start(201, "Created", r#"{"SMSMessageData":{"Recipients":[]}}"#).await;
        let mut config = stub_config(stub.port);
        config.sender_id = String::new();
        AfricasTalkingProvider::new(config)
            .send("+254711123456", "hi there")
            .await
            .expect("send with default sender succeeds");
        let raw = stub.request().await.expect("request captured");
        assert!(!raw.contains("from="), "from omitted when unset: {raw}");
    }

    #[tokio::test]
    #[ignore = "live Africa's Talking delivery — needs NOTIFI_AT_API_KEY, \
                NOTIFI_AT_USERNAME, NOTIFI_AT_SENDER_ID, NOTIFI_AT_TO"]
    async fn live_africas_talking_delivery() {
        let config = AfricasTalkingConfig {
            api_key: std::env::var("NOTIFI_AT_API_KEY").expect("NOTIFI_AT_API_KEY not set"),
            username: std::env::var("NOTIFI_AT_USERNAME").expect("NOTIFI_AT_USERNAME not set"),
            sender_id: std::env::var("NOTIFI_AT_SENDER_ID").expect("NOTIFI_AT_SENDER_ID not set"),
            base_url: None,
        };
        let to = std::env::var("NOTIFI_AT_TO").expect("NOTIFI_AT_TO not set");
        let provider = AfricasTalkingProvider::new(config);
        let body = format!(
            "Notifi live-check {}",
            chrono::Utc::now().timestamp_millis()
        );
        provider
            .send(&to, &body)
            .await
            .expect("real Africa's Talking API accepted the message");
    }
}
