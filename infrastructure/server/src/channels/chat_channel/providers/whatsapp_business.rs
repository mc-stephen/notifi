use futures::future::BoxFuture;
use reqwest::Client;
use serde::Deserialize;

use super::ChatSender;
use crate::channels::chat_channel::ChatMessage;

/// Production Graph API root — unit tests point a local stub at it instead.
#[cfg(test)]
pub(crate) const GRAPH_API_BASE: &str = "https://graph.facebook.com";

/// Default Graph API version when the channel config leaves it unset.
pub const DEFAULT_API_VERSION: &str = "v18.0";

/// WhatsApp Business API configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct WhatsAppBusinessConfig {
    pub phone_number_id: String,
    pub access_token: String,
    #[serde(default)]
    pub api_version: Option<String>,
}

/// WhatsApp Business chat provider controller.
pub struct WhatsAppBusinessProvider {
    config: WhatsAppBusinessConfig,
    client: Client,
    /// Graph root override for tests; production never sets it.
    #[cfg(test)]
    api_base: Option<String>,
}

impl WhatsAppBusinessProvider {
    pub fn new(config: WhatsAppBusinessConfig) -> Self {
        Self {
            config,
            client: Client::new(),
            #[cfg(test)]
            api_base: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_with_api_base(config: WhatsAppBusinessConfig, api_base: String) -> Self {
        Self {
            #[cfg(test)]
            api_base: Some(api_base),
            ..Self::new(config)
        }
    }

    /// Credential check for the provider tester: reads the phone
    /// number's info from Graph — an authenticated read that sends
    /// nothing.
    pub async fn verify(&self) -> Result<(), String> {
        let version = self
            .config
            .api_version
            .as_deref()
            .unwrap_or(DEFAULT_API_VERSION);
        let url = {
            #[cfg(test)]
            {
                match &self.api_base {
                    Some(base) => {
                        format!("{base}/{version}/{}", self.config.phone_number_id)
                    }
                    None => format!("{GRAPH_API_BASE}/{version}/{}", self.config.phone_number_id),
                }
            }
            #[cfg(not(test))]
            {
                format!(
                    "https://graph.facebook.com/{version}/{}",
                    self.config.phone_number_id
                )
            }
        };
        let response = self
            .client
            .get(url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.access_token),
            )
            .send()
            .await
            .map_err(|e| format!("WhatsApp request failed: {e}"))?;
        let status = response.status();
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("WhatsApp response unreadable (HTTP {status}): {e}"))?;
        if status.is_success() {
            Ok(())
        } else {
            let reason = body
                .get("error")
                .and_then(|v| v.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            Err(format!("WhatsApp rejected the credential: {reason}"))
        }
    }
}

impl ChatSender for WhatsAppBusinessProvider {
    fn send(&self, msg: &ChatMessage) -> BoxFuture<'static, Result<(), String>> {
        let version = self
            .config
            .api_version
            .as_deref()
            .unwrap_or(DEFAULT_API_VERSION);
        let url = {
            #[cfg(test)]
            {
                match &self.api_base {
                    Some(base) => {
                        format!("{base}/{version}/{}/messages", self.config.phone_number_id)
                    }
                    None => format!(
                        "{GRAPH_API_BASE}/{version}/{}/messages",
                        self.config.phone_number_id
                    ),
                }
            }
            #[cfg(not(test))]
            {
                format!(
                    "https://graph.facebook.com/{version}/{}/messages",
                    self.config.phone_number_id
                )
            }
        };
        let to = msg.channel_id.clone().unwrap_or_default();
        // The title is the headline — whatsapp's markdown-lite bolds it
        // with single asterisks; the body follows beneath.
        let text = match msg.title.as_deref() {
            Some(title) if !title.is_empty() => format!("*{title}*\n{}", msg.text),
            _ => msg.text.clone(),
        };
        let payload = serde_json::json!({
            "messaging_product": "whatsapp",
            "to": to,
            "type": "text",
            "text": { "body": text }
        });
        let token = self.config.access_token.clone();
        let client = self.client.clone();

        Box::pin(async move {
            let response = client
                .post(&url)
                .header("Authorization", format!("Bearer {token}"))
                .json(&payload)
                .send()
                .await
                .map_err(|e| format!("WhatsApp API request failed: {e}"))?;
            if response.status().is_success() {
                Ok(())
            } else {
                let err = response.text().await.unwrap_or_default();
                Err(format!("WhatsApp API error: {err}"))
            }
        })
    }
}

#[cfg(test)]
mod whatsapp_business_tests {
    use super::*;
    use crate::testing::http_stub;

    const TOKEN: &str = "wa-test-token";

    fn config() -> WhatsAppBusinessConfig {
        WhatsAppBusinessConfig {
            phone_number_id: "PHONE_ID_1".to_string(),
            access_token: TOKEN.to_string(),
            api_version: None,
        }
    }

    fn stub_provider(port: u16) -> WhatsAppBusinessProvider {
        WhatsAppBusinessProvider::new_with_api_base(config(), format!("http://127.0.0.1:{port}"))
    }

    fn msg(to: &str) -> ChatMessage {
        ChatMessage {
            channel_id: Some(to.to_string()),
            title: None,
            text: "hello".to_string(),
            metadata: None,
        }
    }

    #[test]
    fn config_parses_and_defaults_api_version() {
        let c: WhatsAppBusinessConfig =
            serde_json::from_str(r#"{"phone_number_id":"p","access_token":"t"}"#).expect("parses");
        let provider = WhatsAppBusinessProvider::new(c);
        assert_eq!(
            provider.config.api_version.as_deref(),
            None,
            "absent version stays unset"
        );
        let with_version: WhatsAppBusinessConfig = serde_json::from_str(
            r#"{"phone_number_id":"p","access_token":"t","api_version":"v19.0"}"#,
        )
        .expect("parses");
        assert_eq!(with_version.api_version.as_deref(), Some("v19.0"));
    }

    #[tokio::test]
    async fn posts_message_with_bearer_to_graph() {
        let stub = http_stub::start(200, "OK", r#"{"messaging_product":"whatsapp"}"#).await;
        stub_provider(stub.port)
            .send(&msg("+2348012345678"))
            .await
            .expect("whatsapp 200 delivers");
        let raw = stub.request().await.expect("request captured");
        assert!(
            raw.contains(&format!("POST /{DEFAULT_API_VERSION}/PHONE_ID_1/messages")),
            "graph path: {raw}"
        );
        assert!(
            raw.to_ascii_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}")),
            "bearer header: {raw}"
        );
        assert!(
            raw.contains(r#""messaging_product":"whatsapp""#),
            "product body: {raw}"
        );
        assert!(raw.contains(r#""to":"+2348012345678""#), "to body: {raw}");
        assert!(raw.contains(r#""body":"hello""#), "text body: {raw}");
    }

    #[tokio::test]
    async fn surfaces_provider_error_body() {
        let stub = http_stub::start(
            400,
            "Bad Request",
            r#"{"error":{"message":"Invalid parameter: to"}}"#,
        )
        .await;
        let err = stub_provider(stub.port)
            .send(&msg("bad"))
            .await
            .expect_err("400 is a failed WhatsApp send");
        assert!(err.starts_with("WhatsApp API error:"), "prefix: {err}");
        assert!(err.contains("Invalid parameter"), "body surfaced: {err}");
    }

    #[tokio::test]
    async fn title_arrives_bolding_the_first_line() {
        // WhatsApp's markdown-lite bolds with single asterisks.
        let mut titled = msg("+2348012345678");
        titled.title = Some("Deploy done".to_string());
        let stub = http_stub::start(200, "OK", r#"{"messaging_product":"whatsapp"}"#).await;
        stub_provider(stub.port)
            .send(&titled)
            .await
            .expect("titled delivery");
        let raw = stub.request().await.expect("request captured");
        assert!(
            raw.contains(r#""body":"*Deploy done*\nhello""#),
            "bold headline: {raw}"
        );
    }

    #[tokio::test]
    async fn verify_reads_the_phone_info() {
        let stub = http_stub::start(
            200,
            "OK",
            r#"{"display_phone_number":"+1-555-0100","verified_name":"notifi"}"#,
        )
        .await;
        stub_provider(stub.port)
            .verify()
            .await
            .expect("the phone read proves the system-user token");
        let raw = stub.request().await.expect("request captured");
        assert!(
            raw.contains(&format!("GET /{DEFAULT_API_VERSION}/PHONE_ID_1")),
            "phone path: {raw}"
        );
        assert!(
            raw.to_ascii_lowercase()
                .contains(&format!("authorization: bearer {TOKEN}")),
            "bearer header: {raw}"
        );
    }

    #[tokio::test]
    async fn verify_rejects_a_bad_token() {
        let stub = http_stub::start(
            401,
            "Unauthorized",
            r#"{"error":{"message":"Invalid OAuth access token."}}"#,
        )
        .await;
        let err = stub_provider(stub.port)
            .verify()
            .await
            .expect_err("401 rejects the probe");
        assert!(err.contains("WhatsApp rejected the credential"), "{err}");
    }

    #[tokio::test]
    #[ignore = "live WhatsApp delivery — needs NOTIFI_WHATSAPP_ACCESS_TOKEN, \
                NOTIFI_WHATSAPP_PHONE_NUMBER_ID, NOTIFI_WHATSAPP_TO"]
    async fn live_whatsapp_delivery() {
        let config = WhatsAppBusinessConfig {
            phone_number_id: std::env::var("NOTIFI_WHATSAPP_PHONE_NUMBER_ID")
                .expect("NOTIFI_WHATSAPP_PHONE_NUMBER_ID not set"),
            access_token: std::env::var("NOTIFI_WHATSAPP_ACCESS_TOKEN")
                .expect("NOTIFI_WHATSAPP_ACCESS_TOKEN not set"),
            api_version: None,
        };
        let to = std::env::var("NOTIFI_WHATSAPP_TO").expect("NOTIFI_WHATSAPP_TO not set");
        WhatsAppBusinessProvider::new(config)
            .send(&ChatMessage {
                channel_id: Some(to),
                title: None,
                text: format!(
                    "Notifi live-check {}",
                    chrono::Utc::now().timestamp_millis()
                ),
                metadata: None,
            })
            .await
            .expect("real WhatsApp API accepted the message");
    }
}
