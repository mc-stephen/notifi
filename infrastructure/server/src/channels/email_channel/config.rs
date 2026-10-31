use serde::Deserialize;

/// Tagged enum for email provider configuration.
/// The `provider` field in JSON dispatches to the correct config struct.
#[derive(Debug, Deserialize)]
#[serde(tag = "provider")]
pub enum EmailConfig {
    #[serde(rename = "smtp")]
    Smtp(super::smtp::SmtpConfig),
    #[serde(rename = "sendgrid")]
    SendGrid(super::sendgrid::SendGridConfig),
    #[serde(rename = "resend")]
    Resend(super::resend::ResendConfig),
    #[serde(rename = "aws_ses")]
    AwsSes(super::aws_ses::AwsSesConfig),
    #[serde(rename = "postmark")]
    Postmark(super::postmark::PostmarkConfig),
    #[serde(rename = "mailgun")]
    Mailgun(super::mailgun::MailgunConfig),
    #[serde(rename = "brevo")]
    Brevo(super::brevo::BrevoConfig),
}

impl EmailConfig {
    pub fn to_provider(self) -> super::EmailProvider {
        match self {
            Self::Smtp(c) => super::EmailProvider::Smtp(c),
            Self::SendGrid(c) => super::EmailProvider::SendGrid(c),
            Self::Resend(c) => super::EmailProvider::Resend(c),
            Self::AwsSes(c) => super::EmailProvider::AwsSes(c),
            Self::Postmark(c) => super::EmailProvider::Postmark(c),
            Self::Mailgun(c) => super::EmailProvider::Mailgun(c),
            Self::Brevo(c) => super::EmailProvider::Brevo(c),
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
