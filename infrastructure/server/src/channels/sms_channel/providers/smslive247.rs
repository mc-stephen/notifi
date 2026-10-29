use futures::future::BoxFuture;
use reqwest::Client;
use serde::Deserialize;

use super::SmsSender;

/// Production API root; unit tests point `api_base` at a local stub.
pub const API_BASE: &str = "https://api.smslive247.com";

/// SmsLive247 configuration (API v4).
#[derive(Debug, Clone, Deserialize)]
pub struct SmsLive247Config {
    pub api_key: String,
    pub sender_id: String,
    #[serde(default)]
    pub base_url: Option<String>,
}

/// SmsLive247 SMS provider implementation.
///
/// Their v4 answers wrap results in a `data` object; a send echoes the
/// message back (with its messageID) — failures come back as non-2xx
/// HTTP statuses.
pub struct SmsLive247Provider {
    config: SmsLive247Config,
    client: Client,
    /// Test-only API-root override; empty in production.
    api_base: String,
}

impl SmsLive247Provider {
    pub fn new(config: SmsLive247Config) -> Self {
        Self {
            config,
            client: Client::new(),
            api_base: String::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn new_with_base(config: SmsLive247Config, api_base: String) -> Self {
        Self {
            config,
            client: Client::new(),
            api_base,
        }
    }

    fn base(&self) -> String {
        if !self.api_base.is_empty() {
            return self.api_base.clone();
        }
        match &self.config.base_url {
            Some(url) => url.trim_end_matches('/').to_string(),
            None => API_BASE.to_string(),
        }
    }

    /// Connection check for the provider tester: read the current
    /// account — the cheapest authenticated read, no message sent.
    pub async fn verify(&self) -> Result<(), String> {
        let url = format!("{}/api/v4/accounts/self", self.base());
        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await
            .map_err(|e| format!("SmsLive247 request failed: {e}"))?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "SmsLive247 account check returned HTTP {}",
                response.status()
            ))
        }
    }
}

#[derive(Deserialize)]
struct SmsLive247Response {
    #[serde(default)]
    data: SmsLive247Data,
}

#[derive(Deserialize, Default)]
struct SmsLive247Data {
    #[serde(default)]
    message_id: Option<String>,
}

impl SmsSender for SmsLive247Provider {
    fn send(&self, to: &str, text: &str) -> BoxFuture<'static, Result<(), String>> {
        let url = format!("{}/api/v4/sms", self.base());
        // Full international format without the leading '+'.
        let to = to.trim_start_matches('+').to_string();
        let sender = self.config.sender_id.clone();
        let api_key = self.config.api_key.clone();
        let client = self.client.clone();
        let text = text.to_string();

        Box::pin(async move {
            let payload = serde_json::json!({
                "senderID": sender,
                "messageText": text,
                "mobileNumber": to,
            });
            let response = client
                .post(url)
                .header("Authorization", format!("Bearer {api_key}"))
                .json(&payload)
                .send()
                .await
                .map_err(|e| format!("SmsLive247 request failed: {e}"))?;

            let status = response.status();
            let body: SmsLive247Response = response
                .json()
                .await
                .map_err(|e| format!("SmsLive247 response unreadable (HTTP {status}): {e}"))?;
            if status.is_success() {
                Ok(())
            } else {
                Err(format!(
                    "SmsLive247 error: HTTP {} ({})",
                    status,
                    body.data.message_id.unwrap_or_default()
                ))
            }
        })
    }
}

#[cfg(test)]
mod smslive247_tests {
    use super::*;
    use crate::channels::sms_channel::providers::SmsSender;

    const KEY: &str = "MA-key-1";
    const SENDER: &str = "Notifi";

    fn config() -> SmsLive247Config {
        SmsLive247Config {
            api_key: KEY.to_string(),
            sender_id: SENDER.to_string(),
            base_url: None,
        }
    }

    fn stub_provider(port: u16) -> SmsLive247Provider {
        SmsLive247Provider::new_with_base(config(), format!("http://127.0.0.1:{port}"))
    }

    #[test]
    fn config_parses_plain_fields() {
        let c: SmsLive247Config = serde_json::from_str(r#"{"api_key":"k","sender_id":"N"}"#)
            .expect("channel-config JSON deserializes");
        assert_eq!(c.api_key, "k");
        assert_eq!(c.sender_id, "N");
        assert!(c.base_url.is_none());
    }

    #[tokio::test]
    async fn posts_json_with_bearer_and_strips_the_plus() {
        let stub = crate::testing::http_stub::start(
            200,
            "OK",
            r#"{"data":{"messageID":"mid-1","senderID":"Notifi","charged":1}}"#,
        )
        .await;
        stub_provider(stub.port)
            .send("+2348012345678", "welcome")
            .await
            .expect("200 is a delivered SmsLive247 send");
        let raw = stub.request().await.expect("request captured");
        assert!(raw.contains("POST /api/v4/sms"), "endpoint path: {raw}");
        assert!(
            raw.to_ascii_lowercase().contains(&format!(
                "authorization: bearer {}",
                KEY.to_ascii_lowercase()
            )),
            "bearer header: {raw}"
        );
        assert!(
            raw.contains(&format!(r#""senderID":"{SENDER}""#)),
            "sender: {raw}"
        );
        // The leading + is stripped, not percent-encoded.
        assert!(
            raw.contains(r#""mobileNumber":"2348012345678""#),
            "to: {raw}"
        );
        assert!(raw.contains(r#""messageText":"welcome""#), "text: {raw}");
    }

    #[tokio::test]
    async fn surfaces_http_failures() {
        let stub =
            crate::testing::http_stub::start(401, "Unauthorized", r#"{"error":"bad key"}"#).await;
        let err = stub_provider(stub.port)
            .send("+2348012345678", "hi")
            .await
            .expect_err("401 is a failed SmsLive247 send");
        assert!(err.starts_with("SmsLive247 error:"), "{err}");
    }

    #[tokio::test]
    async fn verify_hits_the_account_endpoint() {
        let stub = crate::testing::http_stub::start(
            200,
            "OK",
            r#"{"data":{"acctName":"Ada","acctBal":"103.0"}}"#,
        )
        .await;
        stub_provider(stub.port)
            .verify()
            .await
            .expect("200 account read proves the api key");
        let raw = stub.request().await.expect("request captured");
        assert!(
            raw.contains("GET /api/v4/accounts/self"),
            "probe path: {raw}"
        );
    }

    #[tokio::test]
    #[ignore = "live SmsLive247 delivery — needs NOTIFI_SMSLIVE247_API_KEY, \
                NOTIFI_SMSLIVE247_FROM, NOTIFI_SMSLIVE247_TO"]
    async fn live_smslive247_delivery() {
        let config = SmsLive247Config {
            api_key: std::env::var("NOTIFI_SMSLIVE247_API_KEY")
                .expect("NOTIFI_SMSLIVE247_API_KEY not set"),
            sender_id: std::env::var("NOTIFI_SMSLIVE247_FROM")
                .expect("NOTIFI_SMSLIVE247_FROM not set"),
            base_url: None,
        };
        let to = std::env::var("NOTIFI_SMSLIVE247_TO").expect("NOTIFI_SMSLIVE247_TO not set");
        let provider = SmsLive247Provider::new(config);
        provider
            .send(&to, "Notifi live-check")
            .await
            .expect("real SmsLive247 API accepted the message");
    }
}
