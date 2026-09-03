use serde::Deserialize;
use reqwest::Client;
use futures::future::BoxFuture;

use super::SmsSender;

/// Twilio SMS configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct TwilioConfig {
    pub account_sid: String,
    pub auth_token: String,
    pub from_number: String,
}

/// Twilio SMS provider implementation.
pub struct TwilioProvider {
    config: TwilioConfig,
    client: Client,
}

impl TwilioProvider {
    pub fn new(config: TwilioConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }
}

impl SmsSender for TwilioProvider {
    fn send(&self, to: &str, text: &str) -> BoxFuture<'static, Result<(), String>> {
        let url = format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
            self.config.account_sid
        );
        let from = self.config.from_number.clone();
        let to = to.to_string();
        let text = text.to_string();
        let client = self.client.clone();
        let auth = (self.config.account_sid.clone(), self.config.auth_token.clone());

        Box::pin(async move {
            let params = [("From", from), ("To", to), ("Body", text)];

            let response = client
                .post(url)
                .basic_auth(auth.0, Some(auth.1))
                .form(&params)
                .send()
                .await
                .map_err(|e| format!("Twilio request failed: {e}"))?;

            if response.status().is_success() {
                Ok(())
            } else {
                let err = response.text().await.unwrap_or_default();
                Err(format!("Twilio error: {err}"))
            }
        })
    }
}
