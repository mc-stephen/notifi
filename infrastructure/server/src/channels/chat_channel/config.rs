use serde::Deserialize;

/// Tagged enum for chat provider configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "provider")]
pub enum ChatConfig {
    #[serde(rename = "slack")]
    Slack(super::slack::SlackConfig),
    #[serde(rename = "telegram")]
    Telegram(super::telegram::TelegramConfig),
    #[serde(rename = "whatsapp_business")]
    WhatsAppBusiness(super::whatsapp_business::WhatsAppBusinessConfig),
    #[serde(rename = "discord")]
    Discord(super::discord::DiscordConfig),
    #[serde(rename = "ms_teams")]
    MsTeams(super::ms_teams::MsTeamsConfig),
}

impl ChatConfig {
    pub fn to_provider(self) -> super::ChatProvider {
        match self {
            Self::Slack(c) => super::ChatProvider::Slack(c),
            Self::Telegram(c) => super::ChatProvider::Telegram(c),
            Self::WhatsAppBusiness(c) => super::ChatProvider::WhatsAppBusiness(c),
            Self::Discord(c) => super::ChatProvider::Discord(c),
            Self::MsTeams(c) => super::ChatProvider::MsTeams(c),
        }
    }

    /// Blank-credential guard for the adapter factory: a config whose
    /// credentials are all empty strings parses fine but connects to
    /// nothing, so the factory treats it as provider-missing instead of
    /// wiring the request up to fail mid-send.
    pub fn carries_credentials(&self) -> bool {
        match self {
            Self::Slack(c) => match &c.auth {
                super::slack::SlackAuth::Webhook { webhook_url } => !webhook_url.trim().is_empty(),
                super::slack::SlackAuth::BotApi { bot_token, .. } => !bot_token.trim().is_empty(),
            },
            Self::Telegram(c) => !c.bot_token.trim().is_empty() && !c.chat_id.trim().is_empty(),
            Self::WhatsAppBusiness(c) => {
                !c.phone_number_id.trim().is_empty() && !c.access_token.trim().is_empty()
            }
            Self::Discord(c) => match &c.auth {
                super::discord::DiscordAuth::Webhook { webhook_url } => {
                    !webhook_url.trim().is_empty()
                }
                super::discord::DiscordAuth::BotApi { bot_token, .. } => {
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
