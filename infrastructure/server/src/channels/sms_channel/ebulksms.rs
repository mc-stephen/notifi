use futures::future::BoxFuture;
use reqwest::Client;
use serde::Deserialize;

use super::SmsSender;

/// Production API root; unit tests point `api_base` at a local stub.
pub const API_BASE: &str = "https://api.ebulksms.com";

/// EbulkSms configuration.
///
/// EbulkSms auth is a (username, apikey) pair — the username is the
/// account's email address; the api key is generated in their dashboard.
#[derive(Debug, Clone, Deserialize)]
pub struct EbulkSmsConfig {
    pub username: String,
    pub api_key: String,
    pub sender_id: String,
    #[serde(default)]
    pub base_url: Option<String>,
}

/// EbulkSms SMS provider implementation.
///
/// Verdict rides the body: the response.status string — SUCCESS or a
/// failure code — so HTTP 200 with a failure status is a failed send.
pub struct EbulkSmsProvider {
    config: EbulkSmsConfig,
    client: Client,
    /// Test-only API-root override; empty in production.
    api_base: String,
}

impl EbulkSmsProvider {
    pub fn new(config: EbulkSmsConfig) -> Self {
        Self {
            config,
            client: Client::new(),
            api_base: String::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn new_with_base(config: EbulkSmsConfig, api_base: String) -> Self {
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

    /// Connection check for the provider tester: the balance endpoint —
    /// the cheapest authenticated read, no message sent.
    pub async fn verify(&self) -> Result<(), String> {
        let url = format!(
            "{}/balance/{}/{}",
            self.base(),
            self.config.username,
            self.config.api_key
        );
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("EbulkSms request failed: {e}"))?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "EbulkSms balance check returned HTTP {}",
                response.status()
            ))
        }
    }
}

#[derive(Deserialize)]
struct EbulkResponse {
    #[serde(default)]
    response: EbulkStatus,
}

#[derive(Deserialize, Default)]
struct EbulkStatus {
    #[serde(default)]
    status: String,
}

impl SmsSender for EbulkSmsProvider {
    fn send(&self, to: &str, text: &str) -> BoxFuture<'static, Result<(), String>> {
        let url = format!("{}/sendsms.json", self.base());
        // Their docs want full international format without the '+'.
        let to = to.trim_start_matches('+').to_string();
        let username = self.config.username.clone();
        let api_key = self.config.api_key.clone();
        let sender = self.config.sender_id.clone();
        let client = self.client.clone();
        let text = text.to_string();

        Box::pin(async move {
            let payload = serde_json::json!({
                "SMS": {
                    "auth": {"username": username, "apikey": api_key},
                    "message": {"sender": sender, "messagetext": text, "flash": "0"},
                    "recipients": {"gsm": [{"msidn": to, "msgid": ulid::Ulid::new().to_string()}]},
                    "dndsender": "0",
                }
            });
            let response = client
                .post(url)
                .json(&payload)
                .send()
                .await
                .map_err(|e| format!("EbulkSms request failed: {e}"))?;

            let status = response.status();
            let body: EbulkResponse = response
                .json()
                .await
                .map_err(|e| format!("EbulkSms response unreadable (HTTP {status}): {e}"))?;
            let verdict = body.response.status;
            if status.is_success() && verdict == "SUCCESS" {
                Ok(())
            } else if status.is_success() {
                Err(format!("EbulkSms error: {verdict}"))
            } else {
                Err(format!("EbulkSms error: HTTP {status} ({verdict})"))
            }
        })
    }
}

#[cfg(test)]
mod ebulksms_tests {
    use super::*;
    use crate::channels::sms_channel::SmsSender;

    const USER: &str = "ada@example.com";
    const KEY: &str = "ebulk-key-1";
    const SENDER: &str = "Notifi";

    fn config() -> EbulkSmsConfig {
        EbulkSmsConfig {
            username: USER.to_string(),
            api_key: KEY.to_string(),
            sender_id: SENDER.to_string(),
            base_url: None,
        }
    }

    fn stub_provider(port: u16) -> EbulkSmsProvider {
        EbulkSmsProvider::new_with_base(config(), format!("http://127.0.0.1:{port}"))
    }

    #[test]
    fn config_parses_with_username() {
        let c: EbulkSmsConfig =
            serde_json::from_str(r#"{"username":"a@b.c","api_key":"k","sender_id":"N"}"#)
                .expect("channel-config JSON deserializes");
        assert_eq!(c.username, "a@b.c");
        assert_eq!(c.api_key, "k");
        assert_eq!(c.sender_id, "N");
        assert!(c.base_url.is_none());
    }

    #[tokio::test]
    async fn posts_the_json_envelope_with_auth_and_strips_the_plus() {
        let stub = crate::testing::http_stub::start(
            200,
            "OK",
            r#"{"response":{"status":"SUCCESS","totalsent":1,"cost":1}}"#,
        )
        .await;
        stub_provider(stub.port)
            .send("+2348012345678", "welcome")
            .await
            .expect("SUCCESS is a delivered EbulkSms send");
        let raw = stub.request().await.expect("request captured");
        assert!(raw.contains("POST /sendsms.json"), "endpoint path: {raw}");
        assert!(
            raw.to_ascii_lowercase()
                .contains("content-type: application/json"),
            "json content type: {raw}"
        );
        assert!(
            raw.contains(&format!(r#""username":"{USER}""#)),
            "auth: {raw}"
        );
        assert!(
            raw.contains(&format!(r#""apikey":"{KEY}""#)),
            "apikey: {raw}"
        );
        assert!(
            raw.contains(&format!(r#""sender":"{SENDER}""#)),
            "sender: {raw}"
        );
        // The leading + is stripped; the msgid is minted per recipient.
        assert!(raw.contains(r#""msidn":"2348012345678""#), "msidn: {raw}");
        assert!(raw.contains(r#""msgid""#), "msgid: {raw}");
        assert!(raw.contains(r#""messagetext":"welcome""#), "text: {raw}");
    }

    #[tokio::test]
    async fn http_200_with_failure_status_is_a_failure() {
        let stub = crate::testing::http_stub::start(
            200,
            "OK",
            r#"{"response":{"status":"INSUFFICIENT_CREDIT","totalsent":0,"cost":0}}"#,
        )
        .await;
        let err = stub_provider(stub.port)
            .send("+2348012345678", "hi")
            .await
            .expect_err("INSUFFICIENT_CREDIT is a failed send");
        assert!(err.starts_with("EbulkSms error:"), "prefix: {err}");
        assert!(err.contains("INSUFFICIENT_CREDIT"), "status: {err}");
    }

    #[tokio::test]
    async fn surfaces_http_failures() {
        let stub =
            crate::testing::http_stub::start(401, "Unauthorized", r#"{"response":{}}"#).await;
        let err = stub_provider(stub.port)
            .send("+2348012345678", "hi")
            .await
            .expect_err("401 is a failed EbulkSms send");
        assert!(err.starts_with("EbulkSms error:"), "{err}");
    }

    #[tokio::test]
    async fn verify_hits_the_balance_endpoint() {
        let stub = crate::testing::http_stub::start(200, "OK", "103.0").await;
        stub_provider(stub.port)
            .verify()
            .await
            .expect("200 balance proves the credential pair");
        let raw = stub.request().await.expect("request captured");
        assert!(
            raw.contains(&format!("GET /balance/{USER}/{KEY}")),
            "probe path: {raw}"
        );
    }

    #[tokio::test]
    #[ignore = "live EbulkSms delivery — needs NOTIFI_EBULKSMS_USERNAME, \
                NOTIFI_EBULKSMS_API_KEY, NOTIFI_EBULKSMS_FROM, NOTIFI_EBULKSMS_TO"]
    async fn live_ebulksms_delivery() {
        let config = EbulkSmsConfig {
            username: std::env::var("NOTIFI_EBULKSMS_USERNAME")
                .expect("NOTIFI_EBULKSMS_USERNAME not set"),
            api_key: std::env::var("NOTIFI_EBULKSMS_API_KEY")
                .expect("NOTIFI_EBULKSMS_API_KEY not set"),
            sender_id: std::env::var("NOTIFI_EBULKSMS_FROM").expect("NOTIFI_EBULKSMS_FROM not set"),
            base_url: None,
        };
        let to = std::env::var("NOTIFI_EBULKSMS_TO").expect("NOTIFI_EBULKSMS_TO not set");
        let provider = EbulkSmsProvider::new(config);
        provider
            .send(&to, "Notifi live-check")
            .await
            .expect("real EbulkSms API accepted the message");
    }
}
