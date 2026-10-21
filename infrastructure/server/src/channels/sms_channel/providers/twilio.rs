use futures::future::BoxFuture;
use reqwest::Client;
use serde::Deserialize;

use super::SmsSender;

/// Production API root; unit tests point `api_base` at a local stub.
pub const API_BASE: &str = "https://api.twilio.com";

/// Twilio SMS configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct TwilioConfig {
    pub account_sid: String,
    pub auth_token: String,
    pub from_number: String,
}

/// Twilio SMS provider implementation.
pub struct TwilioProvider {
    config: TwilioConfig,
    client: Client,
    /// API root — production default, stubbed in tests.
    api_base: String,
}

impl TwilioProvider {
    pub fn new(config: TwilioConfig) -> Self {
        Self {
            config,
            client: Client::new(),
            api_base: API_BASE.to_string(),
        }
    }

    #[cfg(test)]
    pub(crate) fn new_with_base(config: TwilioConfig, api_base: String) -> Self {
        Self {
            config,
            client: Client::new(),
            api_base,
        }
    }
}

impl SmsSender for TwilioProvider {
    fn send(&self, to: &str, text: &str) -> BoxFuture<'static, Result<(), String>> {
        let url = format!(
            "{}/2010-04-01/Accounts/{}/Messages.json",
            self.api_base, self.config.account_sid
        );
        let from = self.config.from_number.clone();
        let to = to.to_string();
        let text = text.to_string();
        let client = self.client.clone();
        let auth = (
            self.config.account_sid.clone(),
            self.config.auth_token.clone(),
        );

        Box::pin(async move {
            let params = [("From", from), ("To", to), ("Body", text)];

            let response = client
                .post(url)
                .basic_auth(auth.0, Some(auth.1))
                .form(&params)
                .send()
                .await
                .map_err(|e| format!("Twilio request failed: {e}"))?;

            if response.status().is_success() {
                Ok(())
            } else {
                let err = response.text().await.unwrap_or_default();
                Err(format!("Twilio error: {err}"))
            }
        })
    }
}

#[cfg(test)]
mod twilio_tests {
    use super::*;
    use crate::channels::sms_channel::providers::SmsSender;
    use crate::testing::http_stub;

    const SID: &str = "AC1234567890abcdef";
    const TOKEN: &str = "auth-token-secret";
    const FROM: &str = "+15551234567";

    fn config() -> TwilioConfig {
        TwilioConfig {
            account_sid: SID.to_string(),
            auth_token: TOKEN.to_string(),
            from_number: FROM.to_string(),
        }
    }

    fn stub_provider(port: u16) -> TwilioProvider {
        TwilioProvider::new_with_base(config(), format!("http://127.0.0.1:{port}"))
    }

    #[test]
    fn config_parses_plain_fields() {
        let c: TwilioConfig =
            serde_json::from_str(r#"{"account_sid":"AC1","auth_token":"t","from_number":"+1"}"#)
                .expect("channel-config JSON deserializes");
        assert_eq!(c.account_sid, "AC1");
        assert_eq!(c.auth_token, "t");
        assert_eq!(c.from_number, "+1");
    }

    #[tokio::test]
    async fn posts_form_with_basic_auth_and_accepts_2xx() {
        let stub = http_stub::start(200, "OK", r#"{"sid":"SM100"}"#).await;
        stub_provider(stub.port)
            .send("+2348012345678", "welcome")
            .await
            .expect("200 is a delivered Twilio send");
        let raw = stub.request().await.expect("request captured");
        assert!(
            raw.contains(&format!("POST /2010-04-01/Accounts/{SID}/Messages.json")),
            "message endpoint path: {raw}"
        );
        assert!(
            raw.to_ascii_lowercase().contains("authorization: basic "),
            "basic auth header: {raw}"
        );
        // + previews as %2B in form encoding.
        assert!(
            raw.contains(&format!("From={}", urlencode(FROM))),
            "From field: {raw}"
        );
        assert!(
            raw.contains(&format!("To={}", urlencode("+2348012345678"))),
            "To field: {raw}"
        );
        assert!(raw.contains("Body=welcome"), "Body field: {raw}");
    }

    #[tokio::test]
    async fn surfaces_provider_error_body() {
        let stub =
            http_stub::start(400, "Bad Request", r#"{"code":21211,"message":"invalid"}"#).await;
        let err = stub_provider(stub.port)
            .send("+2348012345678", "hi")
            .await
            .expect_err("400 is a failed Twilio send");
        assert!(err.starts_with("Twilio error:"), "prefix: {err}");
        assert!(err.contains("21211"), "API error body surfaced: {err}");
    }

    fn urlencode(value: &str) -> String {
        // application/x-www-form-urlencoded: the few chars the tests need.
        value.replace('+', "%2B")
    }

    #[tokio::test]
    #[ignore = "live Twilio delivery — needs NOTIFI_TWILIO_ACCOUNT_SID, \
                NOTIFI_TWILIO_AUTH_TOKEN, NOTIFI_TWILIO_FROM, NOTIFI_TWILIO_TO \
                (trial accounts may only text verified numbers)"]
    async fn live_twilio_delivery() {
        let config = TwilioConfig {
            account_sid: std::env::var("NOTIFI_TWILIO_ACCOUNT_SID")
                .expect("NOTIFI_TWILIO_ACCOUNT_SID not set"),
            auth_token: std::env::var("NOTIFI_TWILIO_AUTH_TOKEN")
                .expect("NOTIFI_TWILIO_AUTH_TOKEN not set"),
            from_number: std::env::var("NOTIFI_TWILIO_FROM").expect("NOTIFI_TWILIO_FROM not set"),
        };
        let to = std::env::var("NOTIFI_TWILIO_TO").expect("NOTIFI_TWILIO_TO not set");
        let provider = TwilioProvider::new(config);
        let body = format!(
            "Notifi live-check {}",
            chrono::Utc::now().timestamp_millis()
        );
        provider
            .send(&to, &body)
            .await
            .expect("real Twilio API accepted the message");
    }
}
