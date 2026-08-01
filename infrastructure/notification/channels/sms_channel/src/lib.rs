pub mod config;
pub mod providers;

pub use config::SmsConfig;
use providers::{LocalNigeriaProvider, SmsSender, TwilioProvider};

pub struct SmsMessage {
    pub to: String,
    pub text: String,
}

pub struct SmsProvider {
    sender: Box<dyn SmsSender>,
}

impl SmsProvider {
    pub fn new(config: SmsConfig) -> Self {
        let sender: Box<dyn SmsSender> = match config {
            SmsConfig::Twilio {
                account_sid,
                auth_token,
                from_number,
            } => Box::new(TwilioProvider::new(account_sid, auth_token, from_number)),
            SmsConfig::LocalNigeria {
                api_key,
                base_url,
                sender_id,
            } => Box::new(LocalNigeriaProvider::new(api_key, base_url, sender_id)),
        };
        Self { sender }
    }

    pub async fn send_sms(&self, msg: &SmsMessage) -> Result<(), String> {
        self.sender.send(&msg.to, &msg.text).await
    }
}
