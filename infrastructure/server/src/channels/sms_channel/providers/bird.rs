use futures::future::BoxFuture;
use reqwest::Client;
use serde::Deserialize;

use super::SmsSender;

/// Production API root; unit tests point `api_base` at a local stub.
pub const API_BASE: &str = "https://rest.messagebird.com";

/// Bird (formerly MessageBird, classic SMS REST) configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct BirdConfig {
    pub api_key: String,
    pub base_url: String,
    pub sender_id: String,
}

/// Bird SMS provider implementation.
pub struct BirdProvider {
    config: BirdConfig,
    client: Client,
    /// Test-only API-root override; empty in production — the config's
    /// own `base_url` rules (each tenant carries its endpoint).
    api_base: String,
}

impl BirdProvider {
    pub fn new(config: BirdConfig) -> Self {
        Self {
            config,
            client: Client::new(),
            api_base: String::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn new_with_base(config: BirdConfig, api_base: String) -> Self {
        Self {
            config,
            client: Client::new(),
            api_base,
        }
    }

    /// The saved config carries the tenant's own base_url; tests override.
    fn base(&self) -> String {
        if !self.api_base.is_empty() {
            return self.api_base.clone();
        }
        self.config.base_url.trim_end_matches('/').to_string()
    }

    /// Connection check for the provider tester: hit the balance
    /// endpoint — the cheapest authenticated read, no message sent.
    pub async fn verify(&self) -> Result<(), String> {
        let url = format!("{}/balance", self.base());
        let response = self
            .client
            .get(url)
            .header(
                "Authorization",
                format!("AccessKey {}", self.config.api_key),
            )
            .send()
            .await
            .map_err(|e| format!("Bird request failed: {e}"))?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "Bird balance check returned HTTP {}",
                response.status()
            ))
        }
    }
}

#[derive(Deserialize)]
struct BirdError {
    #[serde(default)]
    errors: Vec<BirdErrorEntry>,
}

#[derive(Deserialize)]
struct BirdErrorEntry {
    #[serde(default)]
    description: Option<String>,
}

impl SmsSender for BirdProvider {
    fn send(&self, to: &str, text: &str) -> BoxFuture<'static, Result<(), String>> {
        let url = format!("{}/messages", self.base());
        // MessageBird wants numbers without the leading '+'.
        let to = to.trim_start_matches('+').to_string();
        let originator = self.config.sender_id.clone();
        let body_text = text.to_string();
        let api_key = self.config.api_key.clone();
        let client = self.client.clone();

        Box::pin(async move {
            let params = [
                ("originator", originator),
                ("recipients", to),
                ("body", body_text),
            ];
            let response = client
                .post(url)
                .header("Authorization", format!("AccessKey {api_key}"))
                .form(&params)
                .send()
                .await
                .map_err(|e| format!("Bird request failed: {e}"))?;

            if response.status().is_success() {
                Ok(())
            } else {
                let parsed: BirdError = response
                    .json()
                    .await
                    .unwrap_or(BirdError { errors: Vec::new() });
                let reason = parsed
                    .errors
                    .into_iter()
                    .next()
                    .and_then(|e| e.description)
                    .unwrap_or_else(|| "unknown error".to_string());
                Err(format!("Bird error: {reason}"))
            }
        })
    }
}

#[cfg(test)]
mod bird_tests {
    use super::*;
    use crate::channels::sms_channel::providers::SmsSender;

    const KEY: &str = "access-key-1";
    const SENDER: &str = "Notifi";

    fn config() -> BirdConfig {
        BirdConfig {
            api_key: KEY.to_string(),
            base_url: API_BASE.to_string(),
            sender_id: SENDER.to_string(),
        }
    }

    fn stub_provider(port: u16) -> BirdProvider {
        BirdProvider::new_with_base(config(), format!("http://127.0.0.1:{port}"))
    }

    #[test]
    fn config_parses_plain_fields() {
        let c: BirdConfig = serde_json::from_str(
            r#"{"api_key":"k","base_url":"https://rest.messagebird.com","sender_id":"N"}"#,
        )
        .expect("channel-config JSON deserializes");
        assert_eq!(c.api_key, "k");
        assert_eq!(c.base_url, "https://rest.messagebird.com");
        assert_eq!(c.sender_id, "N");
    }

    #[tokio::test]
    async fn posts_form_with_access_key_and_strips_the_plus() {
        let stub = crate::testing::http_stub::start(
            200,
            "OK",
            r#"{"id":"mid-1","recipients":{"totalCount":1}}"#,
        )
        .await;
        stub_provider(stub.port)
            .send("+2348012345678", "welcome")
            .await
            .expect("200 is a delivered Bird send");
        let raw = stub.request().await.expect("request captured");
        assert!(raw.contains("POST /messages"), "endpoint path: {raw}");
        assert!(
            raw.to_ascii_lowercase()
                .contains(&format!("authorization: accesskey {KEY}")),
            "access key header: {raw}"
        );
        assert!(
            raw.contains(&format!("originator={SENDER}")),
            "originator: {raw}"
        );
        // The leading + is stripped, not percent-encoded.
        assert!(
            raw.contains("recipients=2348012345678"),
            "recipients: {raw}"
        );
        assert!(raw.contains("body=welcome"), "body: {raw}");
    }

    #[tokio::test]
    async fn surfaces_the_error_description() {
        let stub = crate::testing::http_stub::start(
            422,
            "Unprocessable Entity",
            r#"{"errors":[{"code":9,"description":"originator is invalid"}]}"#,
        )
        .await;
        let err = stub_provider(stub.port)
            .send("+2348012345678", "hi")
            .await
            .expect_err("422 is a failed Bird send");
        assert!(err.starts_with("Bird error:"), "prefix: {err}");
        assert!(err.contains("originator is invalid"), "description: {err}");
    }

    #[tokio::test]
    async fn surfaces_http_failures() {
        let stub = crate::testing::http_stub::start(401, "Unauthorized", r#"{"errors":[]}"#).await;
        let err = stub_provider(stub.port)
            .send("+2348012345678", "hi")
            .await
            .expect_err("401 is a failed Bird send");
        assert!(err.starts_with("Bird error:"), "{err}");
    }

    #[tokio::test]
    async fn verify_hits_the_balance_endpoint() {
        let stub = crate::testing::http_stub::start(200, "OK", r#"{"type":"balance"}"#).await;
        stub_provider(stub.port)
            .verify()
            .await
            .expect("200 balance proves the access key");
        let raw = stub.request().await.expect("request captured");
        assert!(raw.contains("GET /balance"), "probe path: {raw}");
    }

    #[tokio::test]
    #[ignore = "live Bird delivery — needs NOTIFI_BIRD_API_KEY, \
                NOTIFI_BIRD_FROM, NOTIFI_BIRD_TO"]
    async fn live_bird_delivery() {
        let config = BirdConfig {
            api_key: std::env::var("NOTIFI_BIRD_API_KEY").expect("NOTIFI_BIRD_API_KEY not set"),
            base_url: API_BASE.to_string(),
            sender_id: std::env::var("NOTIFI_BIRD_FROM").expect("NOTIFI_BIRD_FROM not set"),
        };
        let to = std::env::var("NOTIFI_BIRD_TO").expect("NOTIFI_BIRD_TO not set");
        let provider = BirdProvider::new(config);
        provider
            .send(&to, "Notifi live-check")
            .await
            .expect("real Bird API accepted the message");
    }
}
