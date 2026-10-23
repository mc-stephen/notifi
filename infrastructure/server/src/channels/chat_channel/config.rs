use serde::Deserialize;

use super::providers;

/// Tagged enum for chat provider configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "provider")]
pub enum ChatConfig {
    #[serde(rename = "slack")]
    Slack(providers::slack::SlackConfig),
    #[serde(rename = "telegram")]
    Telegram(providers::telegram::TelegramConfig),
    #[serde(rename = "whatsapp_business")]
    WhatsAppBusiness(providers::whatsapp_business::WhatsAppBusinessConfig),
    #[serde(rename = "discord")]
    Discord(providers::discord::DiscordConfig),
    #[serde(rename = "ms_teams")]
    MsTeams(providers::ms_teams::MsTeamsConfig),
}

impl ChatConfig {
    pub fn to_provider(self) -> providers::ChatProvider {
        match self {
            Self::Slack(c) => providers::ChatProvider::Slack(c),
            Self::Telegram(c) => providers::ChatProvider::Telegram(c),
            Self::WhatsAppBusiness(c) => providers::ChatProvider::WhatsAppBusiness(c),
            Self::Discord(c) => providers::ChatProvider::Discord(c),
            Self::MsTeams(c) => providers::ChatProvider::MsTeams(c),
        }
    }
}
