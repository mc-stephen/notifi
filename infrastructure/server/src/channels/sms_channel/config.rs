use serde::Deserialize;

/// Tagged enum for SMS provider configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "provider")]
pub enum SmsConfig {
    #[serde(rename = "twilio")]
    Twilio(super::twilio::TwilioConfig),
    #[serde(rename = "local_nigeria")]
    LocalNigeria(super::local_nigeria::LocalNigeriaConfig),
    #[serde(rename = "termii")]
    Termii(super::termii::TermiiConfig),
    #[serde(rename = "africas_talking")]
    AfricasTalking(super::africas_talking::AfricasTalkingConfig),
    #[serde(rename = "smslive247")]
    SmsLive247(super::smslive247::SmsLive247Config),
    #[serde(rename = "ebulksms")]
    EbulkSms(super::ebulksms::EbulkSmsConfig),
    #[serde(rename = "infobip")]
    Infobip(super::infobip::InfobipConfig),
    #[serde(rename = "vonage")]
    Vonage(super::vonage::VonageConfig),
    #[serde(rename = "bird")]
    Bird(super::bird::BirdConfig),
}

impl SmsConfig {
    pub fn to_provider(self) -> super::SmsProvider {
        match self {
            Self::Twilio(c) => super::SmsProvider::Twilio(c),
            Self::LocalNigeria(c) => super::SmsProvider::LocalNigeria(c),
            Self::Termii(c) => super::SmsProvider::Termii(c),
            Self::AfricasTalking(c) => super::SmsProvider::AfricasTalking(c),
            Self::SmsLive247(c) => super::SmsProvider::SmsLive247(c),
            Self::EbulkSms(c) => super::SmsProvider::EbulkSms(c),
            Self::Infobip(c) => super::SmsProvider::Infobip(c),
            Self::Vonage(c) => super::SmsProvider::Vonage(c),
            Self::Bird(c) => super::SmsProvider::Bird(c),
        }
    }
}
