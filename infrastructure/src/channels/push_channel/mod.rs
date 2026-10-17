pub mod config;
pub mod providers;

pub use config::PushConfig;

#[derive(Debug, Clone, serde::Serialize)]
pub struct PushMessage {
    pub token: String,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

pub struct PushSender {
    config: config::PushConfig,
}

impl PushSender {
    pub fn new(config: config::PushConfig) -> Self {
        Self { config }
    }

    pub async fn send_push(&self, _msg: &PushMessage) -> Result<(), String> {
        let provider = self.config.clone().to_provider();
        match provider {
            providers::PushProvider::Fcm(_c) => {
                // TODO: Implement FCM sending
                Err("FCM provider not yet implemented".to_string())
            }
            providers::PushProvider::Apns(_c) => {
                // TODO: Implement APNS sending
                Err("APNS provider not yet implemented".to_string())
            }
            providers::PushProvider::OneSignal(_c) => {
                // TODO: Implement OneSignal sending
                Err("OneSignal provider not yet implemented".to_string())
            }
            providers::PushProvider::Pushy(_c) => {
                // TODO: Implement Pushy sending
                Err("Pushy provider not yet implemented".to_string())
            }
            providers::PushProvider::BrazeAirship(_c) => {
                // TODO: Implement Braze/Airship sending
                Err("Braze/Airship provider not yet implemented".to_string())
            }
        }
    }
}
