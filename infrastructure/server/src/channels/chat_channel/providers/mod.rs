pub mod discord;
pub mod ms_teams;
pub mod slack;
pub mod telegram;
pub mod whatsapp_business;

use futures::future::BoxFuture;

use crate::channels::chat_channel::ChatMessage;

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
