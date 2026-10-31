pub mod config;
pub mod discord;
pub mod ms_teams;
pub mod slack;
pub mod telegram;
pub mod whatsapp_business;

pub use config::ChatConfig;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChatMessage {
    pub channel_id: Option<String>,
    /// Optional headline. Each controller renders it its platform's way
    /// (a bold first line where the platform's plain text allows it, a
    /// plain first line otherwise); `text` is always the body beneath.
    pub title: Option<String>,
    pub text: String,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

use futures::future::BoxFuture;

/// One chat provider controller: config + client in, one HTTP flow out.
/// Controllers know nothing about each other; `ChatMessage` (text plus an
/// optional per-message destination) is the input contract. Error strings
/// surface the provider's own response body — never swallow it.
pub trait ChatSender: Send + Sync {
    fn send(&self, msg: &ChatMessage) -> BoxFuture<'static, Result<(), String>>;
}

/// All supported chat providers. THIS IS THE SOURCE OF TRUTH.
#[derive(Debug, Clone)]
pub enum ChatProvider {
    Slack(slack::SlackConfig),
    Telegram(telegram::TelegramConfig),
    WhatsAppBusiness(whatsapp_business::WhatsAppBusinessConfig),
    Discord(discord::DiscordConfig),
    MsTeams(ms_teams::MsTeamsConfig),
}

impl ChatProvider {
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::Slack(_) => "slack",
            Self::Telegram(_) => "telegram",
            Self::WhatsAppBusiness(_) => "whatsapp_business",
            Self::Discord(_) => "discord",
            Self::MsTeams(_) => "ms_teams",
        }
    }
}

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
            ChatProvider::Slack(c) => slack::SlackProvider::new(c).send(msg).await,
            ChatProvider::Telegram(c) => telegram::TelegramProvider::new(c).send(msg).await,
            ChatProvider::WhatsAppBusiness(c) => {
                whatsapp_business::WhatsAppBusinessProvider::new(c)
                    .send(msg)
                    .await
            }
            ChatProvider::Discord(c) => discord::DiscordProvider::new(c).send(msg).await,
            ChatProvider::MsTeams(c) => ms_teams::MsTeamsProvider::new(c).send(msg).await,
        }
    }
}
