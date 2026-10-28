pub mod config;
pub mod providers;

pub use config::SmsConfig;
use providers::SmsSender;

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
            providers::SmsProvider::Twilio(c) => {
                let sender = providers::twilio::TwilioProvider::new(c);
                sender.send(&msg.to, &msg.text).await
            }
            providers::SmsProvider::LocalNigeria(c) => {
                let sender = providers::local_nigeria::LocalNigeriaProvider::new(c);
                sender.send(&msg.to, &msg.text).await
            }
            providers::SmsProvider::Termii(c) => {
                let sender = providers::termii::TermiiProvider::new(c);
                sender.send(&msg.to, &msg.text).await
            }
            providers::SmsProvider::AfricasTalking(c) => {
                let sender = providers::africas_talking::AfricasTalkingProvider::new(c);
                sender.send(&msg.to, &msg.text).await
            }
            providers::SmsProvider::Vonage(c) => {
                let sender = providers::vonage::VonageProvider::new(c);
                sender.send(&msg.to, &msg.text).await
            }
            providers::SmsProvider::Infobip(c) => {
                let sender = providers::infobip::InfobipProvider::new(c);
                sender.send(&msg.to, &msg.text).await
            }
            providers::SmsProvider::Bird(c) => {
                let sender = providers::bird::BirdProvider::new(c);
                sender.send(&msg.to, &msg.text).await
            }
            providers::SmsProvider::EbulkSms(c) => {
                let sender = providers::ebulksms::EbulkSmsProvider::new(c);
                sender.send(&msg.to, &msg.text).await
            }
            _ => Err(format!(
                "SMS provider not yet implemented: {}",
                provider.provider_name()
            )),
        }
    }
}
