use chrono::{DateTime, Utc, Duration};
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::RwLock;
use std::sync::Arc;

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: i64,
}

pub struct WnsAuthenticator {
    client: Client,
    client_id: String,
    client_secret: String,
    cache: Arc<RwLock<Option<(String, DateTime<Utc>)>>>,
}

impl WnsAuthenticator {
    pub fn new(client_id: String, client_secret: String, _package_sid: String) -> Self {
        Self {
            client: Client::new(),
            client_id,
            client_secret,
            cache: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn get_token(&self) -> Result<String, String> {
        let now = Utc::now();
        
        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some((token, expiry)) = &*cache {
                if *expiry > now + Duration::minutes(5) {
                    return Ok(token.clone());
                }
            }
        }

        // Fetch new token
        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
            ("scope", "notify.windows.com"),
        ];

        let response: TokenResponse = self.client.post("https://login.live.com/accesstoken.srf")
            .form(&params)
            .send().await
            .map_err(|e| format!("Auth request failed: {e}"))?
            .json().await
            .map_err(|e| format!("Failed to parse token response: {e}"))?;

        let expiry = now + Duration::seconds(response.expires_in);
        let token = response.access_token;
        
        // Update cache
        let mut cache = self.cache.write().await;
        *cache = Some((token.clone(), expiry));
        
        Ok(token)
    }
}
