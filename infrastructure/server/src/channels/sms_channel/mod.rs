pub mod africas_talking;
pub mod bird;
pub mod config;
pub mod ebulksms;
pub mod infobip;
pub mod local_nigeria;
pub mod smslive247;
pub mod termii;
pub mod twilio;
pub mod vonage;

pub use config::SmsConfig;

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

pub struct SmsMessage {
    pub to: String,
    pub text: String,
}

pub struct SmsSender2 {
    config: config::SmsConfig,
}

impl SmsSender2 {
    pub fn new(config: config::SmsConfig) -> Self {
        Self { config }
    }

    pub async fn send_sms(&self, msg: &SmsMessage) -> Result<(), String> {
        let provider = self.config.clone().to_provider();
        match provider {
            SmsProvider::Twilio(c) => {
                let sender = twilio::TwilioProvider::new(c);
                sender.send(&msg.to, &msg.text).await
            }
            SmsProvider::LocalNigeria(c) => {
                let sender = local_nigeria::LocalNigeriaProvider::new(c);
                sender.send(&msg.to, &msg.text).await
            }
            SmsProvider::Termii(c) => {
                let sender = termii::TermiiProvider::new(c);
                sender.send(&msg.to, &msg.text).await
            }
            SmsProvider::AfricasTalking(c) => {
                let sender = africas_talking::AfricasTalkingProvider::new(c);
                sender.send(&msg.to, &msg.text).await
            }
            SmsProvider::Vonage(c) => {
                let sender = vonage::VonageProvider::new(c);
                sender.send(&msg.to, &msg.text).await
            }
            SmsProvider::Infobip(c) => {
                let sender = infobip::InfobipProvider::new(c);
                sender.send(&msg.to, &msg.text).await
            }
            SmsProvider::Bird(c) => {
                let sender = bird::BirdProvider::new(c);
                sender.send(&msg.to, &msg.text).await
            }
            SmsProvider::EbulkSms(c) => {
                let sender = ebulksms::EbulkSmsProvider::new(c);
                sender.send(&msg.to, &msg.text).await
            }
            SmsProvider::SmsLive247(c) => {
                let sender = smslive247::SmsLive247Provider::new(c);
                sender.send(&msg.to, &msg.text).await
            }
        }
    }
}
