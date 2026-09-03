use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

use crate::providers;

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
    pub fn load(brand: &str) -> Result<Self, String> {
        let path: PathBuf = ["configs", brand, "chat", "config.json"].iter().collect();
        let bytes = fs::read(&path).map_err(|e| format!("Failed to read {path:?}: {e}"))?;
        serde_json::from_slice::<Self>(&bytes).map_err(|e| format!("Failed to parse config: {e}"))
    }

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
