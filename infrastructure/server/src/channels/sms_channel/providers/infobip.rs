use futures::future::BoxFuture;
use reqwest::Client;
use serde::Deserialize;

use super::SmsSender;

/// Infobip SMS configuration.
///
/// `base_url` is Infobip's per-account endpoint (each tenant gets its
/// own `xxxx.api.infobip.com` host — there is no single global one).
#[derive(Debug, Clone, Deserialize)]
pub struct InfobipConfig {
    pub api_key: String,
    pub base_url: String,
    pub sender_id: String,
}

/// Infobip SMS provider implementation.
///
/// Vonage-style quirk: Infobip accepts the request with HTTP 200 even
/// when a message is rejected — the real verdict rides the body as a
/// per-message `status.groupId` (0 = accepted). The controller parses
/// that body; a bare 2xx is never good enough.
pub struct InfobipProvider {
    config: InfobipConfig,
    client: Client,
    /// API root override for tests; production uses the tenant base_url.
    #[cfg(test)]
    api_base: Option<String>,
}

impl InfobipProvider {
    pub fn new(config: InfobipConfig) -> Self {
        Self {
            config,
            client: Client::new(),
            #[cfg(test)]
            api_base: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_with_base(config: InfobipConfig, api_base: String) -> Self {
        Self {
            config,
            client: Client::new(),
            #[cfg(test)]
            api_base: Some(api_base),
        }
    }

    fn base(&self) -> String {
        #[cfg(test)]
        if let Some(base) = &self.api_base {
            return base.clone();
        }
        self.config.base_url.trim_end_matches('/').to_string()
    }
}

#[derive(Deserialize)]
struct InfobipResponse {
    #[serde(default)]
    messages: Vec<InfobipMessage>,
}

#[derive(Deserialize)]
struct InfobipMessage {
    #[serde(default)]
    status: InfobipStatus,
}

#[derive(Deserialize, Default)]
struct InfobipStatus {
    #[serde(rename = "groupId", default)]
    group_id: i64,
    #[serde(default)]
    description: Option<String>,
}

impl SmsSender for InfobipProvider {
    fn send(&self, to: &str, text: &str) -> BoxFuture<'static, Result<(), String>> {
        let url = format!("{}/sms/2/text/advanced", self.base());
        // Infobip wants destinations without the leading '+'.
        let to = to.trim_start_matches('+').to_string();
        let from = self.config.sender_id.clone();
        let body_text = text.to_string();
        let api_key = self.config.api_key.clone();
        let client = self.client.clone();

        Box::pin(async move {
            let payload = serde_json::json!({
                "messages": [{
                    "from": from,
                    "destinations": [{"to": to}],
                    "text": body_text,
                }]
            });
            let response = client
                .post(url)
                .header("Authorization", format!("Bearer {api_key}"))
                .json(&payload)
                .send()
                .await
                .map_err(|e| format!("Infobip request failed: {e}"))?;

            let status = response.status();
            let body: InfobipResponse = response
                .json()
                .await
                .map_err(|e| format!("Infobip response unreadable (HTTP {status}): {e}"))?;

            let Some(message) = body.messages.into_iter().next() else {
                return Err("Infobip response carried no message verdict".to_string());
            };
            if status.is_success() && message.status.group_id == 0 {
                Ok(())
            } else {
                let reason = message
                    .status
                    .description
                    .unwrap_or_else(|| format!("group {}", message.status.group_id));
                Err(format!("Infobip error: {reason}"))
            }
        })
    }
}

#[cfg(test)]
mod infobip_tests {
    use super::*;
    use crate::channels::sms_channel::providers::SmsSender;

    const KEY: &str = "infobip-key-1";
    const BASE: &str = "https://acct.api.infobip.com";
    const SENDER: &str = "Notifi";

    fn config() -> InfobipConfig {
        InfobipConfig {
            api_key: KEY.to_string(),
            base_url: BASE.to_string(),
            sender_id: SENDER.to_string(),
        }
    }

    fn stub_provider(port: u16) -> InfobipProvider {
        InfobipProvider::new_with_base(config(), format!("http://127.0.0.1:{port}"))
    }

    #[test]
    fn config_parses_plain_fields() {
        let c: InfobipConfig = serde_json::from_str(
            r#"{"api_key":"k","base_url":"https://x.api.infobip.com","sender_id":"N"}"#,
        )
        .expect("channel-config JSON deserializes");
        assert_eq!(c.api_key, "k");
        assert_eq!(c.base_url, "https://x.api.infobip.com");
        assert_eq!(c.sender_id, "N");
    }

    #[tokio::test]
    async fn posts_json_with_bearer_and_strips_the_plus() {
        let stub = crate::testing::http_stub::start(
            200,
            "OK",
            r#"{"messages":[{"to":"2348012345678","status":{"groupId":0,"groupName":"PENDING"},"messageId":"m1"}]}"#,
        )
        .await;
        stub_provider(stub.port)
            .send("+2348012345678", "welcome")
            .await
            .expect("groupId 0 is an accepted Infobip send");
        let raw = stub.request().await.expect("request captured");
        assert!(
            raw.contains("POST /sms/2/text/advanced"),
            "endpoint path: {raw}"
        );
        assert!(
            raw.to_ascii_lowercase()
                .contains(&format!("authorization: bearer {KEY}")),
            "bearer header: {raw}"
        );
        // The leading + is stripped, not percent-encoded.
        assert!(raw.contains(r#""to":"2348012345678""#), "to: {raw}");
        assert!(
            raw.contains(&format!(r#""from":"{SENDER}""#)),
            "from: {raw}"
        );
        assert!(raw.contains(r#""text":"welcome""#), "text: {raw}");
    }

    #[tokio::test]
    async fn http_200_with_rejected_group_is_a_failure() {
        let stub = crate::testing::http_stub::start(
            200,
            "OK",
            r#"{"messages":[{"status":{"groupId":5,"groupName":"REJECTED","description":"Invalid destination address"}}]}"#,
        )
        .await;
        let err = stub_provider(stub.port)
            .send("+2348012345678", "hi")
            .await
            .expect_err("group 5 is a failed Infobip send");
        assert!(err.starts_with("Infobip error:"), "prefix: {err}");
        assert!(err.contains("Invalid destination address"), "error: {err}");
    }

    #[tokio::test]
    async fn surfaces_http_failures() {
        let stub =
            crate::testing::http_stub::start(401, "Unauthorized", r#"{"message":"bad key"}"#).await;
        let err = stub_provider(stub.port)
            .send("+2348012345678", "hi")
            .await
            .expect_err("401 is a failed Infobip send");
        assert!(
            err.starts_with("Infobip error:") || err.contains("Infobip"),
            "{err}"
        );
    }

    #[tokio::test]
    #[ignore = "live Infobip delivery — needs NOTIFI_INFOBIP_API_KEY, \
                NOTIFI_INFOBIP_BASE_URL, NOTIFI_INFOBIP_SENDER_ID, NOTIFI_INFOBIP_TO"]
    async fn live_infobip_delivery() {
        let config = InfobipConfig {
            api_key: std::env::var("NOTIFI_INFOBIP_API_KEY")
                .expect("NOTIFI_INFOBIP_API_KEY not set"),
            base_url: std::env::var("NOTIFI_INFOBIP_BASE_URL")
                .expect("NOTIFI_INFOBIP_BASE_URL not set"),
            sender_id: std::env::var("NOTIFI_INFOBIP_SENDER_ID")
                .expect("NOTIFI_INFOBIP_SENDER_ID not set"),
        };
        let to = std::env::var("NOTIFI_INFOBIP_TO").expect("NOTIFI_INFOBIP_TO not set");
        let provider = InfobipProvider::new(config);
        provider
            .send(&to, "Notifi live-check")
            .await
            .expect("real Infobip API accepted the message");
    }
}
