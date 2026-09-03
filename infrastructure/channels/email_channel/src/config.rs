use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

use crate::providers;

/// Tagged enum for email provider configuration.
/// The `provider` field in JSON dispatches to the correct config struct.
#[derive(Debug, Deserialize)]
#[serde(tag = "provider")]
pub enum EmailConfig {
    #[serde(rename = "smtp")]
    Smtp(providers::smtp::SmtpConfig),
    #[serde(rename = "sendgrid")]
    SendGrid(providers::sendgrid::SendGridConfig),
    #[serde(rename = "resend")]
    Resend(providers::resend::ResendConfig),
    #[serde(rename = "aws_ses")]
    AwsSes(providers::aws_ses::AwsSesConfig),
    #[serde(rename = "postmark")]
    Postmark(providers::postmark::PostmarkConfig),
    #[serde(rename = "mailgun")]
    Mailgun(providers::mailgun::MailgunConfig),
    #[serde(rename = "brevo")]
    Brevo(providers::brevo::BrevoConfig),
}

impl EmailConfig {
    pub fn load(brand: &str) -> Result<Self, String> {
        let path: PathBuf = ["configs", brand, "email", "config.json"].iter().collect();
        let bytes = fs::read(&path).map_err(|e| format!("Failed to read {path:?}: {e}"))?;
        serde_json::from_slice::<Self>(&bytes).map_err(|e| format!("Failed to parse config: {e}"))
    }

    pub fn to_provider(self) -> providers::EmailProvider {
        match self {
            Self::Smtp(c) => providers::EmailProvider::Smtp(c),
            Self::SendGrid(c) => providers::EmailProvider::SendGrid(c),
            Self::Resend(c) => providers::EmailProvider::Resend(c),
            Self::AwsSes(c) => providers::EmailProvider::AwsSes(c),
            Self::Postmark(c) => providers::EmailProvider::Postmark(c),
            Self::Mailgun(c) => providers::EmailProvider::Mailgun(c),
            Self::Brevo(c) => providers::EmailProvider::Brevo(c),
        }
    }

    pub fn from_address(&self) -> &str {
        match self {
            Self::Smtp(c) => &c.from_email,
            Self::SendGrid(c) => &c.from_email,
            Self::Resend(c) => &c.from_email,
            Self::AwsSes(c) => &c.from_email,
            Self::Postmark(c) => &c.from_email,
            Self::Mailgun(c) => &c.from_email,
            Self::Brevo(c) => &c.from_email,
        }
    }

    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::Smtp(_) => "smtp",
            Self::SendGrid(_) => "sendgrid",
            Self::Resend(_) => "resend",
            Self::AwsSes(_) => "aws_ses",
            Self::Postmark(_) => "postmark",
            Self::Mailgun(_) => "mailgun",
            Self::Brevo(_) => "brevo",
        }
    }
}
