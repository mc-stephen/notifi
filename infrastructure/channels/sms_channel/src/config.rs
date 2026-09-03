use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

use crate::providers;

/// Tagged enum for SMS provider configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "provider")]
pub enum SmsConfig {
    #[serde(rename = "twilio")]
    Twilio(providers::twilio::TwilioConfig),
    #[serde(rename = "local_nigeria")]
    LocalNigeria(providers::local_nigeria::LocalNigeriaConfig),
    #[serde(rename = "termii")]
    Termii(providers::termii::TermiiConfig),
    #[serde(rename = "africas_talking")]
    AfricasTalking(providers::africas_talking::AfricasTalkingConfig),
    #[serde(rename = "smslive247")]
    SmsLive247(providers::smslive247::SmsLive247Config),
    #[serde(rename = "ebulksms")]
    EbulkSms(providers::ebulksms::EbulkSmsConfig),
    #[serde(rename = "infobip")]
    Infobip(providers::infobip::InfobipConfig),
    #[serde(rename = "vonage")]
    Vonage(providers::vonage::VonageConfig),
    #[serde(rename = "bird")]
    Bird(providers::bird::BirdConfig),
}

impl SmsConfig {
    pub fn load(brand: &str) -> Result<Self, String> {
        let path: PathBuf = ["configs", brand, "sms", "config.json"].iter().collect();
        let bytes = fs::read(&path).map_err(|e| format!("Failed to read {path:?}: {e}"))?;
        serde_json::from_slice::<Self>(&bytes).map_err(|e| format!("Failed to parse config: {e}"))
    }

    pub fn to_provider(self) -> providers::SmsProvider {
        match self {
            Self::Twilio(c) => providers::SmsProvider::Twilio(c),
            Self::LocalNigeria(c) => providers::SmsProvider::LocalNigeria(c),
            Self::Termii(c) => providers::SmsProvider::Termii(c),
            Self::AfricasTalking(c) => providers::SmsProvider::AfricasTalking(c),
            Self::SmsLive247(c) => providers::SmsProvider::SmsLive247(c),
            Self::EbulkSms(c) => providers::SmsProvider::EbulkSms(c),
            Self::Infobip(c) => providers::SmsProvider::Infobip(c),
            Self::Vonage(c) => providers::SmsProvider::Vonage(c),
            Self::Bird(c) => providers::SmsProvider::Bird(c),
        }
    }
}
