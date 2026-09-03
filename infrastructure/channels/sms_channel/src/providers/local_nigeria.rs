use serde::Deserialize;
use reqwest::Client;
use futures::future::BoxFuture;

use super::SmsSender;

/// Local Nigeria SMS configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct LocalNigeriaConfig {
    pub api_key: String,
    pub base_url: String,
    pub sender_id: String,
}

/// Local Nigeria SMS provider implementation.
pub struct LocalNigeriaProvider {
    config: LocalNigeriaConfig,
    client: Client,
}

impl LocalNigeriaProvider {
    pub fn new(config: LocalNigeriaConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }
}

impl SmsSender for LocalNigeriaProvider {
    fn send(&self, to: &str, text: &str) -> BoxFuture<'static, Result<(), String>> {
        let url = format!("{}/send", self.config.base_url);
        let payload = NigeriaPayload {
            api_key: self.config.api_key.clone(),
            sender_id: self.config.sender_id.clone(),
            to: to.to_string(),
            message: text.to_string(),
        };
        let client = self.client.clone();

        Box::pin(async move {
            let response = client
                .post(url)
                .json(&payload)
                .send()
                .await
                .map_err(|e| format!("Nigeria provider request failed: {e}"))?;

            if response.status().is_success() {
                Ok(())
            } else {
                let err = response.text().await.unwrap_or_default();
                Err(format!("Nigeria provider error: {err}"))
            }
        })
    }
}

#[derive(serde::Serialize)]
struct NigeriaPayload {
    api_key: String,
    sender_id: String,
    to: String,
    message: String,
}
