pub mod discord;
pub mod ms_teams;
pub mod slack;
pub mod telegram;
pub mod whatsapp_business;

use serde::Deserialize;

/// Tagged enum for chat provider configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "provider")]
pub enum ChatConfig {
    #[serde(rename = "slack")]
    Slack(slack::SlackConfig),
    #[serde(rename = "telegram")]
    Telegram(telegram::TelegramConfig),
    #[serde(rename = "whatsapp_business")]
    WhatsAppBusiness(whatsapp_business::WhatsAppBusinessConfig),
    #[serde(rename = "discord")]
    Discord(discord::DiscordConfig),
    #[serde(rename = "ms_teams")]
    MsTeams(ms_teams::MsTeamsConfig),
}

impl ChatConfig {
    pub fn to_provider(self) -> ChatProvider {
        match self {
            Self::Slack(c) => ChatProvider::Slack(c),
            Self::Telegram(c) => ChatProvider::Telegram(c),
            Self::WhatsAppBusiness(c) => ChatProvider::WhatsAppBusiness(c),
            Self::Discord(c) => ChatProvider::Discord(c),
            Self::MsTeams(c) => ChatProvider::MsTeams(c),
        }
    }

    /// Blank-credential guard for the adapter factory: a config whose
    /// credentials are all empty strings parses fine but connects to
    /// nothing, so the factory treats it as provider-missing instead of
    /// wiring the request up to fail mid-send.
    pub fn carries_credentials(&self) -> bool {
        match self {
            Self::Slack(c) => match &c.auth {
                slack::SlackAuth::Webhook { webhook_url } => !webhook_url.trim().is_empty(),
                slack::SlackAuth::BotApi { bot_token, .. } => !bot_token.trim().is_empty(),
            },
            Self::Telegram(c) => !c.bot_token.trim().is_empty() && !c.chat_id.trim().is_empty(),
            Self::WhatsAppBusiness(c) => {
                !c.phone_number_id.trim().is_empty() && !c.access_token.trim().is_empty()
            }
            Self::Discord(c) => match &c.auth {
                discord::DiscordAuth::Webhook { webhook_url } => !webhook_url.trim().is_empty(),
                discord::DiscordAuth::BotApi { bot_token, .. } => !bot_token.trim().is_empty(),
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
    config: ChatConfig,
}

impl ChatSender2 {
    pub fn new(config: ChatConfig) -> Self {
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
