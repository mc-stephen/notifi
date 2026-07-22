use super::WindowSender;
use futures::future::BoxFuture;
use reqwest::Client;
use serde::Serialize;

pub struct WebhookProvider {
    client: Client,
}

impl WebhookProvider {
    pub fn new() -> Self {
        Self { client: Client::new() }
    }
}

#[derive(Serialize)]
struct Payload {
    title: String,
    body: String,
}

impl WindowSender for WebhookProvider {
    fn send(&self, target: &str, title: &str, body: &str) -> BoxFuture<'static, Result<(), String>> {
        let url = target.to_string();
        let payload = Payload { title: title.to_string(), body: body.to_string() };
        let client = self.client.clone();

        Box::pin(async move {
            client.post(url).json(&payload).send().await
                .map_err(|e| format!("Webhook request failed: {e}"))?
                .error_for_status()
                .map_err(|e| format!("Webhook error: {e}"))?;
            Ok(())
        })
    }
}
