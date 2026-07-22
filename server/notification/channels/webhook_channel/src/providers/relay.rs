use super::WebhookSender;
use futures::future::BoxFuture;
use handlebars::Handlebars;
use reqwest::Client;
use std::collections::HashMap;
use chrono::Utc;

pub struct RelayProvider {
    target_url: String,
    method: String,
    headers: HashMap<String, String>,
    template: String,
    client: Client,
    hb: Handlebars<'static>,
}

impl RelayProvider {
    pub fn new(target_url: String, method: String, headers: HashMap<String, String>, template: String) -> Self {
        let mut hb = Handlebars::new();
        hb.register_escape_fn(handlebars::no_escape); // Allow raw JSON
        Self { target_url, method, headers, template, client: Client::new(), hb }
    }
}

impl WebhookSender for RelayProvider {
    fn send(&self, title: &str, body: &str, brand: &str) -> BoxFuture<'static, Result<(), String>> {
        let url = self.target_url.clone();
        let method = self.method.clone();
        let headers = self.headers.clone();
        let template = self.template.clone();
        let client = self.client.clone();
        let hb = self.hb.clone();

        let mut data = HashMap::new();
        data.insert("title", title.to_string());
        data.insert("body", body.to_string());
        data.insert("brand", brand.to_string());
        data.insert("timestamp", Utc::now().to_rfc3339());

        Box::pin(async move {
            let rendered_payload = hb.render_template(&template, &data)
                .map_err(|e| format!("Template rendering error: {e}"))?;

            let mut request = client.request(method.parse().unwrap_or(reqwest::Method::POST), url)
                .body(rendered_payload);

            for (key, value) in headers {
                request = request.header(key, value);
            }

            let response = request.send().await
                .map_err(|e| format!("Relay request failed: {e}"))?;

            if response.status().is_success() {
                Ok(())
            } else {
                let err = response.text().await.unwrap_or_default();
                Err(format!("Relay error: {err}"))
            }
        })
    }
}
