mod config;

pub use config::AndroidConfig;

use reqwest::Client;
use serde::Serialize;
use yup_oauth2::{ServiceAccountAuthenticator, ServiceAccountKey};

pub struct AndroidMessage {
    pub token: String,
    pub title: String,
    pub body: String,
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

pub struct AndroidProvider {
    config: AndroidConfig,
    client: Client,
}

impl AndroidProvider {
    pub fn new(config: AndroidConfig) -> Self {
        Self {
            config,
            client: Client::new(),
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

    pub async fn send_notification(&self, msg: &AndroidMessage) -> Result<(), String> {
        let token = self.get_access_token().await?;
        
        let service_account: serde_json::Value = serde_json::from_reader(
            std::fs::File::open(&self.config.service_account_path).map_err(|e| e.to_string())?
        ).map_err(|e| e.to_string())?;
        
        let project_id = service_account["project_id"]
            .as_str()
            .ok_or("Missing project_id in service account")?;

        let url = format!(
            "https://fcm.googleapis.com/v1/projects/{}/messages:send",
            project_id
        );

        let payload = FcmPayload {
            message: Message {
                token: msg.token.clone(),
                notification: Notification {
                    title: msg.title.clone(),
                    body: msg.body.clone(),
                },
            },
        };

        let response = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", token))
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
}
