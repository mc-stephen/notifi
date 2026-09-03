use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

use crate::providers;

/// Tagged enum for push provider configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "provider")]
pub enum PushConfig {
    #[serde(rename = "fcm")]
    Fcm(providers::fcm::FcmConfig),
    #[serde(rename = "apns")]
    Apns(providers::apns::ApnsConfig),
    #[serde(rename = "onesignal")]
    OneSignal(providers::onesignal::OneSignalConfig),
    #[serde(rename = "pushy")]
    Pushy(providers::pushy::PushyConfig),
    #[serde(rename = "braze_airship")]
    BrazeAirship(providers::braze_airship::BrazeAirshipConfig),
}

impl PushConfig {
    pub fn load(brand: &str) -> Result<Self, String> {
        let path: PathBuf = ["configs", brand, "push", "config.json"].iter().collect();
        let bytes = fs::read(&path).map_err(|e| format!("Failed to read {path:?}: {e}"))?;
        serde_json::from_slice::<Self>(&bytes).map_err(|e| format!("Failed to parse config: {e}"))
    }

    pub fn to_provider(self) -> providers::PushProvider {
        match self {
            Self::Fcm(c) => providers::PushProvider::Fcm(c),
            Self::Apns(c) => providers::PushProvider::Apns(c),
            Self::OneSignal(c) => providers::PushProvider::OneSignal(c),
            Self::Pushy(c) => providers::PushProvider::Pushy(c),
            Self::BrazeAirship(c) => providers::PushProvider::BrazeAirship(c),
        }
    }
}
