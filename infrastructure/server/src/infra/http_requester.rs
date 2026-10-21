//! Outbound HTTP driver for flow HTTP Request steps (reqwest).

use crate::ports::http_requester::{BoxHttpFut, HttpCall, HttpRequester, HttpResult};

pub struct ReqwestHttpRequester {
    client: reqwest::Client,
}

impl ReqwestHttpRequester {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for ReqwestHttpRequester {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpRequester for ReqwestHttpRequester {
    fn request(&self, call: &HttpCall) -> BoxHttpFut<'_, Result<HttpResult, String>> {
        let client = self.client.clone();
        let call = call.clone();
        Box::pin(async move {
            let method = reqwest::Method::from_bytes(call.method.as_bytes())
                .map_err(|_| format!("unsupported HTTP method '{}'", call.method))?;
            let mut request = client.request(method, &call.url);
            for (key, value) in &call.headers {
                request = request.header(key, value);
            }
            if !call.body.is_empty() {
                request = request.body(call.body.clone());
            }
            let response = request.send().await.map_err(|e| e.to_string())?;
            let status = response.status().as_u16();
            Ok(HttpResult {
                status,
                ok: response.status().is_success(),
            })
        })
    }
}
