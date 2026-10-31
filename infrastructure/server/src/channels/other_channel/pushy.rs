use reqwest::Client;
use serde::Deserialize;

/// Pushy configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct PushyConfig {
    pub api_key: String,
    #[serde(default)]
    pub app_id: Option<String>,
}

/// Production API root; unit tests point `api_base` at a local stub.
pub const API_BASE: &str = "https://api.pushy.me";

/// Pushy device push controller.
///
/// The verdict rides the body: Pushy answers a send with 200 and
/// success true; failures are non-2xx with a code + error pair.
/// The api key travels as the query parameter their docs require.
pub struct PushyProvider {
    config: PushyConfig,
    client: Client,
    /// Test-only API-root override; empty in production.
    api_base: String,
}

impl PushyProvider {
    pub fn new(config: PushyConfig) -> Self {
        Self {
            config,
            api_base: String::new(),
            client: Client::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn new_with_base(config: PushyConfig, api_base: String) -> Self {
        Self {
            config,
            api_base,
            client: Client::new(),
        }
    }

    fn base(&self) -> String {
        if !self.api_base.is_empty() {
            return self.api_base.clone();
        }
        API_BASE.to_string()
    }

    /// Credential check for the provider tester: reads the Device Info
    /// API for a synthetic token. Pushy's error codes distinguish the
    /// verdicts — INVALID_API_KEY rejects the credential; a missing /
    /// mismatched device means the key itself is live. Nothing is sent.
    pub async fn verify(&self) -> Result<(), String> {
        let url = format!("{}/devices/notifi-credential-probe", self.base());
        let response = self
            .client
            .get(url)
            .query(&[("api_key", self.config.api_key.as_str())])
            .send()
            .await
            .map_err(|e| format!("Pushy request failed: {e}"))?;

        let body: PushyError = response.json().await.unwrap_or(PushyError::default());

        match body.code.as_str() {
            "INVALID_API_KEY" => Err(format!(
                "Pushy rejected the credential: {}",
                body.error.unwrap_or_else(|| "unspecified".to_string())
            )),
            // Any other verdict (device not found, app mismatch, or a
            // read that went through) proves the API key is live.
            _ => Ok(()),
        }
    }
}

#[derive(Deserialize)]
struct PushyResponse {
    #[serde(default)]
    success: bool,
}

#[derive(Deserialize, Default)]
struct PushyError {
    #[serde(default)]
    code: String,
    #[serde(default)]
    error: Option<String>,
}

impl PushyProvider {
    /// The android send shape: posts notification + data in the same
    /// docs-recommended envelope.
    pub async fn send_notification(&self, msg: &PushyMessage) -> Result<(), String> {
        let url = self.base();
        let api_key = self.config.api_key.clone();
        let client = self.client.clone();
        let token = msg.token.clone();
        let title = msg.title.clone();
        let body = msg.body.clone();

        let payload = serde_json::json!({
            "to": token,
            "data": {"message": body},
            "notification": {"title": title, "body": body},
        });
        let response = client
            .post(format!("{url}/push"))
            .query(&[("api_key", api_key.as_str())])
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Pushy request failed: {e}"))?;

        let status = response.status();
        if status.is_success() {
            let verdict: PushyResponse = response
                .json()
                .await
                .map_err(|e| format!("Pushy response unreadable (HTTP {status}): {e}"))?;
            if verdict.success {
                Ok(())
            } else {
                Err("Pushy error: the response did not confirm success".to_string())
            }
        } else {
            let error: PushyError = response.json().await.unwrap_or_default();
            Err(format!(
                "Pushy error: {}",
                error.error.unwrap_or_else(|| error.code.clone())
            ))
        }
    }
}

/// Android destination send shape: token, title, body.
pub struct PushyMessage {
    pub token: String,
    pub title: String,
    pub body: String,
}

#[cfg(test)]
mod pushy_tests {
    use super::*;

    const KEY: &str = "pushy-key-1";

    fn config() -> PushyConfig {
        PushyConfig {
            api_key: KEY.to_string(),
            app_id: None,
        }
    }

    fn stub_provider(port: u16) -> PushyProvider {
        PushyProvider::new_with_base(config(), format!("http://127.0.0.1:{port}"))
    }

    #[test]
    fn config_parses_with_optional_app_id() {
        let c: PushyConfig = serde_json::from_str(r#"{"api_key":"k"}"#).expect("parses");
        assert_eq!(c.api_key, "k");
        assert!(c.app_id.is_none());
        let c: PushyConfig =
            serde_json::from_str(r#"{"api_key":"k","app_id":"a1"}"#).expect("parses");
        assert_eq!(c.app_id.as_deref(), Some("a1"));
    }

    #[tokio::test]
    async fn posts_json_with_the_api_key_query_param() {
        let stub = crate::testing::http_stub::start(200, "OK", r#"{"success":true}"#).await;
        stub_provider(stub.port)
            .send_notification(&PushyMessage {
                token: "tok-1".to_string(),
                title: "Hi".to_string(),
                body: "welcome".to_string(),
            })
            .await
            .expect("success true is a delivered Pushy send");
        let raw = stub.request().await.expect("request captured");
        assert!(raw.contains("POST /push"), "endpoint path: {raw}");
        assert!(raw.contains(&format!("api_key={KEY}")), "query key: {raw}");
        assert!(
            raw.to_ascii_lowercase()
                .contains("content-type: application/json"),
            "json content type: {raw}"
        );
        assert!(raw.contains(r#""to":"tok-1""#), "to: {raw}");
        assert!(raw.contains(r#""title":"Hi""#), "title: {raw}");
        assert!(raw.contains(r#""body":"welcome""#), "body: {raw}");
        assert!(raw.contains(r#""message":"welcome""#), "data: {raw}");
    }

    #[tokio::test]
    async fn unconfirmed_success_is_a_failure() {
        let stub = crate::testing::http_stub::start(200, "OK", r#"{"success":false}"#).await;
        let err = stub_provider(stub.port)
            .send_notification(&PushyMessage {
                token: "tok-1".to_string(),
                title: "Hi".to_string(),
                body: "b".to_string(),
            })
            .await
            .expect_err("success false is a failed Pushy send");
        assert!(err.starts_with("Pushy error:"), "{err}");
    }

    #[tokio::test]
    async fn http_failures_surface_the_code() {
        let stub = crate::testing::http_stub::start(
            403,
            "Forbidden",
            r#"{"code":"INVALID_API_KEY","error":"The Secret API Key you used is invalid."}"#,
        )
        .await;
        let err = stub_provider(stub.port)
            .send_notification(&PushyMessage {
                token: "tok-1".to_string(),
                title: "Hi".to_string(),
                body: "b".to_string(),
            })
            .await
            .expect_err("403 is a failed Pushy send");
        assert!(err.starts_with("Pushy error:"), "prefix: {err}");
        assert!(err.contains("The Secret API Key"), "error surfaced: {err}");
    }

    #[tokio::test]
    async fn verify_proves_the_key_on_a_missing_device() {
        // A live key reading a synthetic token gets the device-not-found
        // verdict — that proves the credential without any state change.
        let stub = crate::testing::http_stub::start(
            400,
            "Bad Request",
            r#"{"code":"INVALID_DEVICE_TOKEN","error":"We could not find a device with that token."}"#,
        )
        .await;
        stub_provider(stub.port)
            .verify()
            .await
            .expect("the key is live");
        let raw = stub.request().await.expect("request captured");
        assert!(
            raw.contains("GET /devices/notifi-credential-probe"),
            "probe path: {raw}"
        );
    }

    #[tokio::test]
    async fn verify_rejects_a_bad_key() {
        let stub = crate::testing::http_stub::start(
            403,
            "Forbidden",
            r#"{"code":"INVALID_API_KEY","error":"invalid"}"#,
        )
        .await;
        let err = stub_provider(stub.port)
            .verify()
            .await
            .expect_err("an INVALID_API_KEY verdict rejects");
        assert!(err.contains("Pushy rejected the credential"), "{err}");
    }

    #[tokio::test]
    #[ignore = "live Pushy delivery — needs NOTIFI_PUSHY_API_KEY and \
                a real device token: NOTIFI_PUSHY_DEVICE_TOKEN"]
    async fn live_pushy_delivery() {
        let provider = PushyProvider::new(PushyConfig {
            api_key: std::env::var("NOTIFI_PUSHY_API_KEY").expect("NOTIFI_PUSHY_API_KEY not set"),
            app_id: None,
        });
        let token =
            std::env::var("NOTIFI_PUSHY_DEVICE_TOKEN").expect("NOTIFI_PUSHY_DEVICE_TOKEN not set");
        provider
            .send_notification(&PushyMessage {
                token,
                title: "Notifi live-check".to_string(),
                body: "Real Pushy delivery check.".to_string(),
            })
            .await
            .expect("real Pushy API accepted the push");
    }
}
