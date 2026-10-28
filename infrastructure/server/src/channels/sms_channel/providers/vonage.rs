use futures::future::BoxFuture;
use reqwest::Client;
use serde::Deserialize;

use super::SmsSender;

/// Production API root; unit tests point `api_base` at a local stub.
pub const API_BASE: &str = "https://rest.nexmo.com";

/// Vonage (formerly Nexmo) SMS configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct VonageConfig {
    pub api_key: String,
    pub api_secret: String,
    pub from_number: String,
}

/// Vonage SMS provider implementation.
///
/// Vonage quirk: the HTTP status is almost always 200 — the real verdict
/// rides the body as a per-message `status` (0 = sent, anything else
/// fails with `error-text`). The controller parses that body; a bare 2xx
/// is never good enough.
pub struct VonageProvider {
    config: VonageConfig,
    client: Client,
    /// API root — production default, stubbed in tests.
    api_base: String,
}

impl VonageProvider {
    pub fn new(config: VonageConfig) -> Self {
        Self {
            config,
            client: Client::new(),
            api_base: API_BASE.to_string(),
        }
    }

    #[cfg(test)]
    pub(crate) fn new_with_base(config: VonageConfig, api_base: String) -> Self {
        Self {
            config,
            client: Client::new(),
            api_base,
        }
    }
}

#[derive(Deserialize)]
struct VonageResponse {
    #[serde(default)]
    messages: Vec<VonageMessage>,
}

#[derive(Deserialize)]
struct VonageMessage {
    #[serde(default)]
    status: String,
    #[serde(rename = "error-text", default)]
    error_text: Option<String>,
}

impl SmsSender for VonageProvider {
    fn send(&self, to: &str, text: &str) -> BoxFuture<'static, Result<(), String>> {
        let url = format!("{}/sms/json", self.api_base);
        // Vonage wants E.164 without the leading '+'.
        let to = to.trim_start_matches('+').to_string();
        let from = self.config.from_number.clone();
        let key = self.config.api_key.clone();
        let secret = self.config.api_secret.clone();
        let body_text = text.to_string();
        let client = self.client.clone();

        Box::pin(async move {
            let params = [
                ("api_key", key),
                ("api_secret", secret),
                ("from", from),
                ("to", to),
                ("text", body_text),
            ];
            let response = client
                .post(url)
                .form(&params)
                .send()
                .await
                .map_err(|e| format!("Vonage request failed: {e}"))?;

            let status = response.status();
            let body: VonageResponse = response
                .json()
                .await
                .map_err(|e| format!("Vonage response unreadable (HTTP {status}): {e}"))?;

            let Some(message) = body.messages.into_iter().next() else {
                return Err("Vonage response carried no message verdict".to_string());
            };
            if status.is_success() && message.status == "0" {
                Ok(())
            } else {
                let reason = message
                    .error_text
                    .unwrap_or_else(|| format!("status {}", message.status));
                Err(format!("Vonage error: {reason}"))
            }
        })
    }
}

#[cfg(test)]
mod vonage_tests {
    use super::*;
    use crate::channels::sms_channel::providers::SmsSender;

    const KEY: &str = "vkey1";
    const SECRET: &str = "vsecret1";
    const FROM: &str = "Notifi";

    fn config() -> VonageConfig {
        VonageConfig {
            api_key: KEY.to_string(),
            api_secret: SECRET.to_string(),
            from_number: FROM.to_string(),
        }
    }

    fn stub_provider(port: u16) -> VonageProvider {
        VonageProvider::new_with_base(config(), format!("http://127.0.0.1:{port}"))
    }

    const fn ok_body() -> &'static str {
        r#"{"message-count":"1","messages":[{"to":"2348012345678","status":"0","message-id":"SM-1"}]}"#
    }

    #[test]
    fn config_parses_plain_fields() {
        let c: VonageConfig =
            serde_json::from_str(r#"{"api_key":"k","api_secret":"s","from_number":"Notifi"}"#)
                .expect("channel-config JSON deserializes");
        assert_eq!(c.api_key, "k");
        assert_eq!(c.api_secret, "s");
        assert_eq!(c.from_number, "Notifi");
    }

    #[tokio::test]
    async fn posts_form_with_creds_and_strips_the_plus() {
        let stub = http_stub_start(ok_body()).await;
        stub_provider(stub.port)
            .send("+2348012345678", "welcome")
            .await
            .expect("status 0 is a delivered Vonage send");
        let raw = stub.request().await.expect("request captured");
        assert!(raw.contains("POST /sms/json"), "endpoint path: {raw}");
        assert!(raw.contains(&format!("api_key={KEY}")), "key param: {raw}");
        assert!(
            raw.contains(&format!("api_secret={SECRET}")),
            "secret param: {raw}"
        );
        assert!(raw.contains(&format!("from={FROM}")), "from param: {raw}");
        // The leading + is stripped, not percent-encoded.
        assert!(raw.contains("to=2348012345678"), "to param: {raw}");
        assert!(
            raw.contains("Body=welcome") || raw.contains("text=welcome"),
            "body: {raw}"
        );
    }

    #[tokio::test]
    async fn http_200_with_nonzero_status_is_a_failure() {
        // Vonage's quirk: happy HTTP status, failed message verdict.
        let stub = http_stub_start(
            r#"{"message-count":"1","messages":[{"status":"9","error-text":"Invalid message payload"}]}"#,
        )
        .await;
        let err = stub_provider(stub.port)
            .send("+2348012345678", "hi")
            .await
            .expect_err("status 9 is a failed Vonage send");
        assert!(err.starts_with("Vonage error:"), "prefix: {err}");
        assert!(err.contains("Invalid message payload"), "error text: {err}");
    }

    #[tokio::test]
    async fn surfaces_http_failures() {
        // A 401 with an unreadable verdict body still fails clearly.
        let stub =
            crate::testing::http_stub::start(401, "Unauthorized", r#"{"error":"401"}"#).await;
        let err =
            VonageProvider::new_with_base(config(), format!("http://127.0.0.1:{}", stub.port))
                .send("+2348012345678", "hi")
                .await;
        assert!(err.is_err());
    }

    #[tokio::test]
    #[ignore = "live Vonage delivery — needs NOTIFI_VONAGE_API_KEY, \
                NOTIFI_VONAGE_API_SECRET, NOTIFI_VONAGE_FROM, NOTIFI_VONAGE_TO"]
    async fn live_vonage_delivery() {
        let config = VonageConfig {
            api_key: std::env::var("NOTIFI_VONAGE_API_KEY").expect("NOTIFI_VONAGE_API_KEY not set"),
            api_secret: std::env::var("NOTIFI_VONAGE_API_SECRET")
                .expect("NOTIFI_VONAGE_API_SECRET not set"),
            from_number: std::env::var("NOTIFI_VONAGE_FROM").expect("NOTIFI_VONAGE_FROM not set"),
        };
        let to = std::env::var("NOTIFI_VONAGE_TO").expect("NOTIFI_VONAGE_TO not set");
        let provider = VonageProvider::new(config);
        provider
            .send(&to, "Notifi live-check")
            .await
            .expect("real Vonage API accepted the message");
    }

    // The http_stub lives in the crate's testing module; each test owns
    // one stub instance (200 OK + the verdict body).
    async fn http_stub_start(body: &'static str) -> crate::testing::http_stub::Stub {
        crate::testing::http_stub::start(200, "OK", body).await
    }
}
