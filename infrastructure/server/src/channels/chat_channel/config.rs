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

    /// Blank-credential guard for the adapter factory: a config whose
    /// credentials are all empty strings parses fine but connects to
    /// nothing, so the factory treats it as provider-missing instead of
    /// wiring the request up to fail mid-send.
    pub fn carries_credentials(&self) -> bool {
        match self {
            Self::Slack(c) => match &c.auth {
                providers::slack::SlackAuth::Webhook { webhook_url } => {
                    !webhook_url.trim().is_empty()
                }
                providers::slack::SlackAuth::BotApi { bot_token, .. } => {
                    !bot_token.trim().is_empty()
                }
            },
            Self::Telegram(c) => !c.bot_token.trim().is_empty() && !c.chat_id.trim().is_empty(),
            Self::WhatsAppBusiness(c) => {
                !c.phone_number_id.trim().is_empty() && !c.access_token.trim().is_empty()
            }
            Self::Discord(c) => match &c.auth {
                providers::discord::DiscordAuth::Webhook { webhook_url } => {
                    !webhook_url.trim().is_empty()
                }
                providers::discord::DiscordAuth::BotApi { bot_token, .. } => {
                    !bot_token.trim().is_empty()
                }
            },
            Self::MsTeams(c) => !c.webhook_url.trim().is_empty(),
        }
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn telegram_blank_credentials_do_not_count() {
        let blank: ChatConfig =
            serde_json::from_str(r#"{"provider":"telegram","bot_token":"","chat_id":"c"}"#)
                .expect("parses");
        assert!(!blank.carries_credentials());
        let full: ChatConfig =
            serde_json::from_str(r#"{"provider":"telegram","bot_token":"b","chat_id":"c"}"#)
                .expect("parses");
        assert!(full.carries_credentials());
    }

    #[test]
    fn webhook_flavors_need_their_url() {
        let blank: ChatConfig =
            serde_json::from_str(r#"{"provider":"ms_teams","webhook_url":""}"#).expect("parses");
        assert!(!blank.carries_credentials());
        let full: ChatConfig = serde_json::from_str(
            r#"{"provider":"ms_teams","webhook_url":"https://outlook.office.com/webhook/x"}"#,
        )
        .expect("parses");
        assert!(full.carries_credentials());
    }
}
