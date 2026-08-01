use super::SmsSender;
use futures::future::BoxFuture;
use reqwest::Client;

pub struct TwilioProvider {
    account_sid: String,
    auth_token: String,
    from_number: String,
    client: Client,
}

impl TwilioProvider {
    pub fn new(account_sid: String, auth_token: String, from_number: String) -> Self {
        Self {
            account_sid,
            auth_token,
            from_number,
            client: Client::new(),
        }
    }
}

impl SmsSender for TwilioProvider {
    fn send(&self, to: &str, text: &str) -> BoxFuture<'static, Result<(), String>> {
        let url = format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
            self.account_sid
        );
        let from = self.from_number.clone();
        let to = to.to_string();
        let text = text.to_string();
        let client = self.client.clone();
        let auth = (self.account_sid.clone(), self.auth_token.clone());

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
