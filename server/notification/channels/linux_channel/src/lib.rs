mod config;

pub use config::LinuxConfig;

use reqwest::Client;
use serde::Serialize;
use std::collections::HashMap;

pub struct LinuxMessage {
    pub endpoint: String,
    pub title: String,
    pub body: String,
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Serialize)]
struct LinuxPayload {
    title: String,
    body: String,
}

pub struct LinuxProvider {
    _config: LinuxConfig,
    client: Client,
}

impl LinuxProvider {
    pub fn new(config: LinuxConfig) -> Self {
        Self {
            _config: config,
            client: Client::new(),
        }
    }

    pub async fn send_notification(&self, msg: &LinuxMessage) -> Result<(), String> {
        let payload = LinuxPayload {
            title: msg.title.clone(),
            body: msg.body.clone(),
        };

        let mut request = self.client.post(&msg.endpoint).json(&payload);

        if let Some(headers) = &msg.headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Linux webhook request failed: {e}"))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let err = response.text().await.unwrap_or_default();
            Err(format!("Linux webhook error: {err}"))
        }
    }
}
