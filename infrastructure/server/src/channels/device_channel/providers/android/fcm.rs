use super::AndroidMessage;
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use std::path::PathBuf;
use yup_oauth2::ServiceAccountAuthenticator;

/// FCM configuration for the Android destination.
///
/// `service_account_path` points at the Google service account JSON whose
/// `project_id` names the FCM project; both the OAuth token and the
/// project id are read from that one file.
#[derive(Debug, Clone, Deserialize)]
pub struct FcmConfig {
    pub service_account_path: PathBuf,
}

#[derive(Serialize)]
struct FcmPayload {
    message: Message,
}

#[derive(Serialize)]
struct Message {
    token: String,
    notification: Notification,
}

#[derive(Serialize)]
struct Notification {
    title: String,
    body: String,
}

/// Android (FCM) device provider controller.
pub struct FcmProvider {
    config: FcmConfig,
    client: Client,
    /// FCM root override for tests; production never sets it.
    #[cfg(test)]
    fcm_base: Option<String>,
}

impl FcmProvider {
    pub fn new(config: FcmConfig) -> Self {
        Self {
            config,
            client: Client::new(),
            #[cfg(test)]
            fcm_base: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_with_fcm_base(config: FcmConfig, fcm_base: String) -> Self {
        Self {
            #[cfg(test)]
            fcm_base: Some(fcm_base),
            ..Self::new(config)
        }
    }

    async fn get_access_token(&self) -> Result<String, String> {
        let secret = yup_oauth2::read_service_account_key(&self.config.service_account_path)
            .await
            .map_err(|e| format!("Failed to read service account key: {e}"))?;

        let auth = ServiceAccountAuthenticator::builder(secret)
            .build()
            .await
            .map_err(|e| format!("Failed to build authenticator: {e}"))?;

        let token = auth
            .token(&["https://www.googleapis.com/auth/firebase.messaging"])
            .await
            .map_err(|e| format!("Failed to get token: {e}"))?;

        Ok(token.token().ok_or("No token in response")?.to_string())
    }

    /// The FCM POST itself — split from token acquisition so tests can
    /// drive the wire shape without Google's OAuth service.
    pub(crate) async fn post_fcm(
        &self,
        token: &str,
        project_id: &str,
        msg: &AndroidMessage,
    ) -> Result<(), String> {
        let url = {
            #[cfg(test)]
            {
                match &self.fcm_base {
                    Some(base) => format!("{base}/v1/projects/{project_id}/messages:send"),
                    None => {
                        format!("https://fcm.googleapis.com/v1/projects/{project_id}/messages:send")
                    }
                }
            }
            #[cfg(not(test))]
            {
                format!("https://fcm.googleapis.com/v1/projects/{project_id}/messages:send")
            }
        };
        let payload = FcmPayload {
            message: Message {
                token: msg.token.clone(),
                notification: Notification {
                    title: msg.title.clone(),
                    body: msg.body.clone(),
                },
            },
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("FCM request failed: {e}"))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let err = response.text().await.unwrap_or_default();
            Err(format!("FCM error: {err}"))
        }
    }

    /// The project id embedded in the service account file.
    pub(crate) fn project_id(&self) -> Result<String, String> {
        let service_account: serde_json::Value = serde_json::from_reader(
            std::fs::File::open(&self.config.service_account_path).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;

        service_account["project_id"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| "Missing project_id in service account".to_string())
    }

    pub async fn send_notification(&self, msg: &AndroidMessage) -> Result<(), String> {
        let token = self.get_access_token().await?;
        let project_id = self.project_id()?;
        self.post_fcm(&token, &project_id, msg).await
    }
}

#[cfg(test)]
mod android_tests {
    use super::*;

    fn config() -> FcmConfig {
        FcmConfig {
            service_account_path: std::path::PathBuf::from("/nonexistent/sa.json"),
        }
    }

    #[test]
    fn config_parses_service_account_path() {
        let c: FcmConfig =
            serde_json::from_str(r#"{"service_account_path":"/keys/fcm.json"}"#).expect("parses");
        assert_eq!(c.service_account_path.to_str(), Some("/keys/fcm.json"));
    }

    #[tokio::test]
    async fn posts_fcm_payload_with_bearer() {
        let stub = crate::testing::http_stub::start(200, "OK", r#"{"name":"m1"}"#).await;
        let provider =
            FcmProvider::new_with_fcm_base(config(), format!("http://127.0.0.1:{}", stub.port));
        provider
            .post_fcm(
                "fake-oauth-token",
                "proj-1",
                &AndroidMessage {
                    token: "device-token-1".to_string(),
                    title: "Hi".to_string(),
                    body: "Body".to_string(),
                },
            )
            .await
            .expect("FCM 200 delivers");
        let raw = stub.request().await.expect("request captured");
        assert!(
            raw.contains("POST /v1/projects/proj-1/messages:send"),
            "fcm path: {raw}"
        );
        assert!(
            raw.to_ascii_lowercase()
                .contains("authorization: bearer fake-oauth-token"),
            "bearer header: {raw}"
        );
        assert!(raw.contains(r#""token":"device-token-1""#), "token: {raw}");
        assert!(raw.contains(r#""body":"Body""#), "body: {raw}");
    }

    #[tokio::test]
    async fn surfaces_fcm_error_body() {
        let stub = crate::testing::http_stub::start(
            400,
            "Bad Request",
            r#"{"error":{"code":400,"message":"The registration token is invalid"}}"#,
        )
        .await;
        let provider =
            FcmProvider::new_with_fcm_base(config(), format!("http://127.0.0.1:{}", stub.port));
        let err = provider
            .post_fcm(
                "t",
                "proj-1",
                &AndroidMessage {
                    token: "bad".to_string(),
                    title: "x".to_string(),
                    body: "y".to_string(),
                },
            )
            .await
            .expect_err("400 is a failed FCM send");
        assert!(err.starts_with("FCM error:"), "prefix: {err}");
        assert!(
            err.contains("registration token is invalid"),
            "error body surfaced: {err}"
        );
    }

    #[tokio::test]
    #[ignore = "live FCM delivery — needs a real service account JSON and NOTIFI_ANDROID_DEVICE_TOKEN"]
    async fn live_android_delivery() {
        let path = std::env::var("NOTIFI_ANDROID_SERVICE_ACCOUNT")
            .expect("NOTIFI_ANDROID_SERVICE_ACCOUNT not set");
        let device_token = std::env::var("NOTIFI_ANDROID_DEVICE_TOKEN")
            .expect("NOTIFI_ANDROID_DEVICE_TOKEN not set");
        FcmProvider::new(FcmConfig {
            service_account_path: std::path::PathBuf::from(path),
        })
        .send_notification(&AndroidMessage {
            token: device_token,
            title: "Notifi live-check".to_string(),
            body: "Direct FCM delivery check".to_string(),
        })
        .await
        .expect("real FCM API accepted the push");
    }
}
