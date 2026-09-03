pub mod twilio;
pub mod local_nigeria;
pub mod termii;
pub mod africas_talking;
pub mod smslive247;
pub mod ebulksms;
pub mod infobip;
pub mod vonage;
pub mod bird;

use futures::future::BoxFuture;

/// All supported SMS providers. THIS IS THE SOURCE OF TRUTH.
#[derive(Debug, Clone)]
pub enum SmsProvider {
    Twilio(twilio::TwilioConfig),
    LocalNigeria(local_nigeria::LocalNigeriaConfig),
    Termii(termii::TermiiConfig),
    AfricasTalking(africas_talking::AfricasTalkingConfig),
    SmsLive247(smslive247::SmsLive247Config),
    EbulkSms(ebulksms::EbulkSmsConfig),
    Infobip(infobip::InfobipConfig),
    Vonage(vonage::VonageConfig),
    Bird(bird::BirdConfig),
}

impl SmsProvider {
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::Twilio(_) => "twilio",
            Self::LocalNigeria(_) => "local_nigeria",
            Self::Termii(_) => "termii",
            Self::AfricasTalking(_) => "africas_talking",
            Self::SmsLive247(_) => "smslive247",
            Self::EbulkSms(_) => "ebulksms",
            Self::Infobip(_) => "infobip",
            Self::Vonage(_) => "vonage",
            Self::Bird(_) => "bird",
        }
    }
}

pub trait SmsSender: Send + Sync {
    fn send(&self, to: &str, text: &str) -> BoxFuture<'static, Result<(), String>>;
}
