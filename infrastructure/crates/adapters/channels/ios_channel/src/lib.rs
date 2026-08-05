mod config;

pub use config::IosConfig;

use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use reqwest::Client;
use serde::Serialize;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct IosMessage {
    pub device_token: String,
    pub title: String,
    pub body: String,
}

#[derive(Serialize)]
struct ApnsPayload {
    aps: Aps,
}

#[derive(Serialize)]
struct Aps {
    alert: Alert,
}

#[derive(Serialize)]
struct Alert {
    title: String,
    body: String,
}

#[derive(Serialize)]
struct Claims {
    iss: String,
    iat: u64,
}

pub struct IosProvider {
    config: IosConfig,
    client: Client,
}

impl IosProvider {
    pub fn new(config: IosConfig) -> Self {
        Self {
            config,
            client: Client::builder().http2_prior_knowledge().build().unwrap(),
        }
    }

    fn generate_jwt(&self) -> Result<String, String> {
        let p8_key = fs::read_to_string(&self.config.p8_key_path)
            .map_err(|e| format!("Failed to read p8 key: {e}"))?;

        let header = Header::new(Algorithm::ES256);
        let claims = Claims {
            iss: self.config.team_id.clone(),
            iat: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let encoding_key = EncodingKey::from_ec_pem(p8_key.as_bytes())
            .map_err(|e| format!("Failed to create encoding key: {e}"))?;

        encode(&header, &claims, &encoding_key).map_err(|e| format!("Failed to encode JWT: {e}"))
    }

    pub async fn send_notification(&self, msg: &IosMessage) -> Result<(), String> {
        let jwt = self.generate_jwt()?;
        let url = format!("https://api.push.apple.com/3/device/{}", msg.device_token);

        let payload = ApnsPayload {
            aps: Aps {
                alert: Alert {
                    title: msg.title.clone(),
                    body: msg.body.clone(),
                },
            },
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", jwt))
            .header("apns-topic", &self.config.bundle_id)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("APNs request failed: {e}"))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let err = response.text().await.unwrap_or_default();
            Err(format!("APNs error: {err}"))
        }
    }
}
