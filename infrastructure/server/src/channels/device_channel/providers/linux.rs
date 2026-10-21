use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;

/// Linux configuration.
///
/// The generic channel holds no settings of its own yet — the endpoint
/// arrives per message.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct LinuxConfig {
    // Generic channel, config may be empty for now or store global settings
}

/// Linux destination: a self-hosted webhook endpoint (optionally with
/// per-message auth headers).
pub struct LinuxMessage {
    pub endpoint: String,
    pub title: String,
    pub body: String,
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Serialize)]
struct LinuxPayload {
    title: String,
    body: String,
}

/// Linux device provider controller — endpoint delivery.
pub struct LinuxProvider {
    _config: LinuxConfig,
    client: Client,
}

impl LinuxProvider {
    pub fn new(config: LinuxConfig) -> Self {
        Self {
            _config: config,
            client: Client::new(),
        }
    }

    pub async fn send_notification(&self, msg: &LinuxMessage) -> Result<(), String> {
        let payload = LinuxPayload {
            title: msg.title.clone(),
            body: msg.body.clone(),
        };

        let mut request = self.client.post(&msg.endpoint).json(&payload);

        if let Some(headers) = &msg.headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Linux webhook request failed: {e}"))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let err = response.text().await.unwrap_or_default();
            Err(format!("Linux webhook error: {err}"))
        }
    }
}

#[cfg(test)]
mod linux_tests {
    use super::*;

    #[test]
    fn config_parses_empty_object() {
        let c: LinuxConfig = serde_json::from_str("{}").expect("empty object parses");
        let _ = LinuxProvider::new(c);
    }

    #[tokio::test]
    async fn posts_payload_with_per_message_headers() {
        let stub = crate::testing::http_stub::start(200, "OK", "").await;
        let mut headers = HashMap::new();
        headers.insert("X-Device-Key".to_string(), "linux-key-1".to_string());
        LinuxProvider::new(LinuxConfig::default())
            .send_notification(&LinuxMessage {
                endpoint: format!("http://127.0.0.1:{}/notify", stub.port),
                title: "Hi".to_string(),
                body: "Body".to_string(),
                headers: Some(headers),
            })
            .await
            .expect("linux endpoint 200 delivers");
        let raw = stub.request().await.expect("request captured");
        assert!(raw.contains("POST /notify"), "endpoint path: {raw}");
        assert!(
            raw.contains("x-device-key: linux-key-1"),
            "per-message header: {raw}"
        );
        assert!(raw.contains(r#""title":"Hi""#), "payload: {raw}");
        assert!(raw.contains(r#""body":"Body""#), "payload: {raw}");
    }

    #[tokio::test]
    async fn surfaces_endpoint_error_body() {
        let stub = crate::testing::http_stub::start(500, "Server Error", "not-a-daemon").await;
        let err = LinuxProvider::new(LinuxConfig::default())
            .send_notification(&LinuxMessage {
                endpoint: format!("http://127.0.0.1:{}/notify", stub.port),
                title: "t".to_string(),
                body: "b".to_string(),
                headers: None,
            })
            .await
            .expect_err("500 is a failed linux delivery");
        assert!(err.starts_with("Linux webhook error:"), "prefix: {err}");
        assert!(err.contains("not-a-daemon"), "body surfaced: {err}");
    }
}
