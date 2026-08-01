use super::SmsSender;
use futures::future::BoxFuture;
use reqwest::Client;
use serde::Serialize;

pub struct LocalNigeriaProvider {
    api_key: String,
    base_url: String,
    sender_id: String,
    client: Client,
}

impl LocalNigeriaProvider {
    pub fn new(api_key: String, base_url: String, sender_id: String) -> Self {
        Self {
            api_key,
            base_url,
            sender_id,
            client: Client::new(),
        }
    }
}

#[derive(Serialize)]
struct NigeriaPayload {
    api_key: String,
    sender_id: String,
    to: String,
    message: String,
}

impl SmsSender for LocalNigeriaProvider {
    fn send(&self, to: &str, text: &str) -> BoxFuture<'static, Result<(), String>> {
        let url = format!("{}/send", self.base_url);
        let payload = NigeriaPayload {
            api_key: self.api_key.clone(),
            sender_id: self.sender_id.clone(),
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
