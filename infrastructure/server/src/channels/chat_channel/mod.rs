pub mod config;
pub mod providers;

pub use config::ChatConfig;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChatMessage {
    pub channel_id: Option<String>,
    pub text: String,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

use providers::ChatSender;

pub struct ChatSender2 {
    config: config::ChatConfig,
}

impl ChatSender2 {
    pub fn new(config: config::ChatConfig) -> Self {
        Self { config }
    }

    /// Dispatches to the configured provider's controller. Controllers own
    /// their whole HTTP flow; this match is just routing.
    pub async fn send_message(&self, msg: &ChatMessage) -> Result<(), String> {
        let provider = self.config.clone().to_provider();
        match provider {
            providers::ChatProvider::Slack(c) => {
                providers::slack::SlackProvider::new(c).send(msg).await
            }
            providers::ChatProvider::Telegram(c) => {
                providers::telegram::TelegramProvider::new(c)
                    .send(msg)
                    .await
            }
            providers::ChatProvider::WhatsAppBusiness(c) => {
                providers::whatsapp_business::WhatsAppBusinessProvider::new(c)
                    .send(msg)
                    .await
            }
            providers::ChatProvider::Discord(c) => {
                providers::discord::DiscordProvider::new(c).send(msg).await
            }
            providers::ChatProvider::MsTeams(c) => {
                providers::ms_teams::MsTeamsProvider::new(c).send(msg).await
            }
        }
    }
}
