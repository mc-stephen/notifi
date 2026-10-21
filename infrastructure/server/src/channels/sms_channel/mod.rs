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
            _ => Err(format!(
                "SMS provider not yet implemented: {}",
                provider.provider_name()
            )),
        }
    }
}
