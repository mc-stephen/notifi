pub mod slack;
pub mod telegram;
pub mod whatsapp_business;
pub mod discord;
pub mod ms_teams;

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
