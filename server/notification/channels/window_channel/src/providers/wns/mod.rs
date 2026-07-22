mod auth;
mod xml;

use super::WindowSender;
use auth::WnsAuthenticator;
use xml::build_toast_xml;
use futures::future::BoxFuture;
use reqwest::Client;
use std::sync::Arc;

pub struct WnsProvider {
    auth: Arc<WnsAuthenticator>,
    client: Client,
}

impl WnsProvider {
    pub fn new(client_id: String, client_secret: String, package_sid: String) -> Self {
        Self {
            auth: Arc::new(WnsAuthenticator::new(client_id, client_secret, package_sid)),
            client: Client::new(),
        }
    }
}

impl WindowSender for WnsProvider {
    fn send(&self, target: &str, title: &str, body: &str) -> BoxFuture<'static, Result<(), String>> {
        let auth = self.auth.clone();
        let client = self.client.clone();
        let uri = target.to_string();
        let xml = build_toast_xml(title, body);

        Box::pin(async move {
            let token = auth.get_token().await?;

            let response = client.post(uri)
                .header("Authorization", format!("Bearer {}", token))
                .header("X-WNS-Type", "wns/toast")
                .header("Content-Type", "text/xml")
                .body(xml)
                .send()
                .await
                .map_err(|e| format!("WNS request failed: {e}"))?;

            if response.status().is_success() {
                Ok(())
            } else {
                let err = response.text().await.unwrap_or_default();
                Err(format!("WNS error: {err}"))
            }
        })
    }
}
