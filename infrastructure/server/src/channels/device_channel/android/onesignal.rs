use reqwest::Client;
use serde::Deserialize;

/// OneSignal configuration.
///
/// The dashboard collects both keys; OneSignal's notification API
/// authenticates with the REST API key, so the controller prefers an
/// explicit `rest_api_key` and falls back to `api_key` (the list of
/// keys is the tenant's call, the wire protocol is ours).
#[derive(Debug, Clone, Deserialize)]
pub struct OneSignalConfig {
    pub app_id: String,
    pub api_key: String,
    #[serde(default)]
    pub rest_api_key: Option<String>,
}

/// Production API root; unit tests point `api_base` at a local stub.
pub const API_BASE: &str = "https://onesignal.com";

/// OneSignal device push controller.
///
/// One vendor serves both android and apple destinations — the same
/// REST call delivers either platform, so one controller is wired for
/// both channel pairs.
pub struct OneSignalProvider {
    config: OneSignalConfig,
    client: Client,
    /// Test-only API-root override; empty in production.
    api_base: String,
}

impl OneSignalProvider {
    pub fn new(config: OneSignalConfig) -> Self {
        Self {
            config,
            client: Client::new(),
            api_base: String::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn new_with_base(config: OneSignalConfig, api_base: String) -> Self {
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
        API_BASE.to_string()
    }

    fn auth_key(&self) -> &str {
        match &self.config.rest_api_key {
            Some(key) if !key.is_empty() => key,
            _ => &self.config.api_key,
        }
    }

    /// Credential check for the provider tester: lists the app's
    /// notifications (limit 1) — an authenticated read that sends
    /// nothing.
    pub async fn verify(&self) -> Result<(), String> {
        let url = format!("{}/api/v1/notifications", self.base());
        let response = self
            .client
            .get(url)
            .query(&[("app_id", self.config.app_id.as_str()), ("limit", "1")])
            .header("Authorization", format!("Basic {}", self.auth_key()))
            .send()
            .await
            .map_err(|e| format!("OneSignal request failed: {e}"))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let body: OneSignalError = response.json().await.unwrap_or_default();
            Err(format!(
                "OneSignal rejected the credential: {}",
                body.first_error()
                    .unwrap_or_else(|| "unknown error".to_string())
            ))
        }
    }
}

/// Android push: the token is a subscription id; apple uses the same
/// call.
pub struct OneSignalMessage {
    pub token: String,
    pub title: String,
    pub body: String,
}

#[derive(Deserialize, Default)]
struct OneSignalError {
    #[serde(default)]
    errors: Vec<String>,
}

impl OneSignalError {
    fn first_error(&self) -> Option<String> {
        self.errors.first().cloned()
    }
}

impl OneSignalProvider {
    pub async fn send_notification(&self, msg: &OneSignalMessage) -> Result<(), String> {
        let url = format!("{}/api/v1/notifications", self.base());
        let app_id = self.config.app_id.clone();
        let auth = self.auth_key().to_string();
        let token = msg.token.clone();
        let title = msg.title.clone();
        let body = msg.body.clone();

        let payload = serde_json::json!({
            "app_id": app_id,
            "include_subscription_ids": [token],
            "target_channel": "push",
            "headings": {"en": title},
            "contents": {"en": body},
        });
        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Basic {auth}"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("OneSignal request failed: {e}"))?;

        let status = response.status();
        let verdict: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("OneSignal response unreadable (HTTP {status}): {e}"))?;
        let id = verdict.get("id").and_then(|v| v.as_str()).unwrap_or("");
        if status.is_success() && !id.is_empty() {
            Ok(())
        } else {
            // Rejections carry an empty id plus an errors array.
            let reason = verdict
                .get("errors")
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .and_then(|e| e.as_str())
                .unwrap_or("empty notification id — check the app id and key");
            Err(format!("OneSignal error: {reason}"))
        }
    }
}

#[cfg(test)]
mod onesignal_tests {
    use super::*;

    const APP: &str = "app-id-1";
    const KEY: &str = "rest-key-1";

    fn config() -> OneSignalConfig {
        OneSignalConfig {
            app_id: APP.to_string(),
            api_key: "plain-key".to_string(),
            rest_api_key: Some(KEY.to_string()),
        }
    }

    fn stub_provider(port: u16) -> OneSignalProvider {
        OneSignalProvider::new_with_base(config(), format!("http://127.0.0.1:{port}"))
    }

    #[test]
    fn config_prefers_the_rest_key_for_auth() {
        let c: OneSignalConfig =
            serde_json::from_str(r#"{"app_id":"a","api_key":"plain","rest_api_key":"rest"}"#)
                .expect("channel-config JSON deserializes");
        let provider = OneSignalProvider::new(c);
        assert_eq!(provider.auth_key(), "rest");
        // And falls back to api_key when the rest key is absent.
        let c: OneSignalConfig =
            serde_json::from_str(r#"{"app_id":"a","api_key":"plain"}"#).expect("parses");
        assert_eq!(OneSignalProvider::new(c).auth_key(), "plain");
    }

    #[tokio::test]
    async fn posts_targets_and_bodies_with_basic_auth() {
        let stub =
            crate::testing::http_stub::start(200, "OK", r#"{"id":"notif-1","recipients":1}"#).await;
        stub_provider(stub.port)
            .send_notification(&OneSignalMessage {
                token: "sub-1".to_string(),
                title: "Hi".to_string(),
                body: "welcome".to_string(),
            })
            .await
            .expect("an id back is a delivered OneSignal send");
        let raw = stub.request().await.expect("request captured");
        assert!(raw.contains("POST /api/v1/notifications"), "path: {raw}");
        assert!(
            raw.to_ascii_lowercase().contains(&format!(
                "authorization: basic {}",
                KEY.to_ascii_lowercase()
            )),
            "basic header: {raw}"
        );
        assert!(
            raw.contains(&format!(r#""app_id":"{APP}""#)),
            "app id: {raw}"
        );
        assert!(
            raw.contains(r#""include_subscription_ids":["sub-1"]"#),
            "target: {raw}"
        );
        assert!(raw.contains(r#""headings":{"en":"Hi"}"#), "title: {raw}");
        assert!(
            raw.contains(r#""contents":{"en":"welcome"}"#),
            "body: {raw}"
        );
    }

    #[tokio::test]
    async fn empty_id_200_is_a_failure() {
        let stub = crate::testing::http_stub::start(200, "OK", r#"{"id":"","errors":[]}"#).await;
        let err = stub_provider(stub.port)
            .send_notification(&OneSignalMessage {
                token: "sub-1".to_string(),
                title: "Hi".to_string(),
                body: "b".to_string(),
            })
            .await
            .expect_err("an empty id on 200 is a failed send");
        assert!(err.starts_with("OneSignal error:"), "{err}");
    }

    #[tokio::test]
    async fn surfaces_http_failures() {
        let stub = crate::testing::http_stub::start(
            401,
            "Unauthorized",
            r#"{"errors":["Please include a case-sensitive header of Authorization: Basic"]}"#,
        )
        .await;
        let err = stub_provider(stub.port)
            .send_notification(&OneSignalMessage {
                token: "sub-1".to_string(),
                title: "Hi".to_string(),
                body: "b".to_string(),
            })
            .await
            .expect_err("401 is a failed OneSignal send");
        assert!(err.starts_with("OneSignal error:"), "{err}");
    }

    #[tokio::test]
    async fn verify_reads_the_notification_list() {
        let stub = crate::testing::http_stub::start(200, "OK", r#"{"notifications":[]}"#).await;
        stub_provider(stub.port)
            .verify()
            .await
            .expect("200 list proves the app id + rest key");
        let raw = stub.request().await.expect("request captured");
        assert!(
            raw.contains("GET /api/v1/notifications?app_id=app-id-1&limit=1"),
            "probe query: {raw}"
        );
    }

    #[tokio::test]
    async fn verify_rejects_bad_credentials() {
        let stub =
            crate::testing::http_stub::start(401, "Unauthorized", r#"{"errors":["invalid key"]}"#)
                .await;
        let err = stub_provider(stub.port)
            .verify()
            .await
            .expect_err("401 rejects");
        assert!(err.contains("OneSignal rejected the credential"), "{err}");
    }

    #[tokio::test]
    #[ignore = "live OneSignal delivery — needs NOTIFI_ONESIGNAL_APP_ID, \
                NOTIFI_ONESIGNAL_REST_API_KEY, and a real subscription id: \
                NOTIFI_ONESIGNAL_SUBSCRIPTION_ID"]
    async fn live_onesignal_delivery() {
        let provider = OneSignalProvider::new(OneSignalConfig {
            app_id: std::env::var("NOTIFI_ONESIGNAL_APP_ID")
                .expect("NOTIFI_ONESIGNAL_APP_ID not set"),
            api_key: String::new(),
            rest_api_key: Some(
                std::env::var("NOTIFI_ONESIGNAL_REST_API_KEY")
                    .expect("NOTIFI_ONESIGNAL_REST_API_KEY not set"),
            ),
        });
        let token = std::env::var("NOTIFI_ONESIGNAL_SUBSCRIPTION_ID")
            .expect("NOTIFI_ONESIGNAL_SUBSCRIPTION_ID not set");
        provider
            .send_notification(&OneSignalMessage {
                token,
                title: "Notifi live-check".to_string(),
                body: "Real OneSignal delivery check.".to_string(),
            })
            .await
            .expect("real OneSignal API accepted the push");
    }
}
