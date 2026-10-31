pub mod aws_ses;
pub mod brevo;
pub mod mailgun;
pub mod postmark;
pub mod resend;
pub mod sendgrid;
pub mod smtp;

use serde::Deserialize;

/// Tagged enum for email provider configuration.
/// The `provider` field in JSON dispatches to the correct config struct.
#[derive(Debug, Deserialize)]
#[serde(tag = "provider")]
pub enum EmailConfig {
    #[serde(rename = "smtp")]
    Smtp(smtp::SmtpConfig),
    #[serde(rename = "sendgrid")]
    SendGrid(sendgrid::SendGridConfig),
    #[serde(rename = "resend")]
    Resend(resend::ResendConfig),
    #[serde(rename = "aws_ses")]
    AwsSes(aws_ses::AwsSesConfig),
    #[serde(rename = "postmark")]
    Postmark(postmark::PostmarkConfig),
    #[serde(rename = "mailgun")]
    Mailgun(mailgun::MailgunConfig),
    #[serde(rename = "brevo")]
    Brevo(brevo::BrevoConfig),
}

impl EmailConfig {
    pub fn to_provider(self) -> EmailProvider {
        match self {
            Self::Smtp(c) => EmailProvider::Smtp(c),
            Self::SendGrid(c) => EmailProvider::SendGrid(c),
            Self::Resend(c) => EmailProvider::Resend(c),
            Self::AwsSes(c) => EmailProvider::AwsSes(c),
            Self::Postmark(c) => EmailProvider::Postmark(c),
            Self::Mailgun(c) => EmailProvider::Mailgun(c),
            Self::Brevo(c) => EmailProvider::Brevo(c),
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

/// All supported email providers. THIS IS THE SOURCE OF TRUTH.
/// To add a new provider: create new_provider.rs, add variant here.
#[derive(Debug, Clone)]
pub enum EmailProvider {
    Smtp(smtp::SmtpConfig),
    SendGrid(sendgrid::SendGridConfig),
    Resend(resend::ResendConfig),
    AwsSes(aws_ses::AwsSesConfig),
    Postmark(postmark::PostmarkConfig),
    Mailgun(mailgun::MailgunConfig),
    Brevo(brevo::BrevoConfig),
}

impl EmailProvider {
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

use lettre::{
    Address, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
};

pub struct EmailMessage {
    pub from_email: String,
    pub from_name: Option<String>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub reply_to: Option<String>,
    pub subject: String,
    pub body_text: String,
    pub body_html: Option<String>,
}

pub struct EmailSender {
    config: EmailConfig,
}

impl EmailSender {
    pub fn new(config: EmailConfig) -> Self {
        Self { config }
    }

    fn from_name(config: &EmailConfig) -> Option<String> {
        match config {
            EmailConfig::Smtp(c) => c.from_name.clone(),
            EmailConfig::SendGrid(c) => c.from_name.clone(),
            EmailConfig::Resend(c) => c.from_name.clone(),
            EmailConfig::AwsSes(c) => c.from_name.clone(),
            EmailConfig::Postmark(c) => c.from_name.clone(),
            EmailConfig::Mailgun(c) => c.from_name.clone(),
            EmailConfig::Brevo(c) => c.from_name.clone(),
        }
    }

    /// Handshake + auth check only (SMTP NOOP via
    /// `AsyncSmtpTransport::test_connection`). No message is sent.
    /// `Ok(true)` = the relay accepted us; `Ok(false)` = connected but the
    /// relay refused NOOP; `Err` carries the SMTP error (e.g. 535 auth).
    pub async fn verify(&self) -> Result<bool, String> {
        match &self.config {
            EmailConfig::Smtp(smtp_config) => Ok(Self::smtp_transport(smtp_config)?
                .test_connection()
                .await
                .map_err(|e| format!("SMTP verify failed: {e}"))?),
            _ => Err(format!(
                "provider {} has no connection check",
                self.config.provider_name()
            )),
        }
    }

    fn smtp_transport(
        smtp_config: &smtp::SmtpConfig,
    ) -> Result<AsyncSmtpTransport<Tokio1Executor>, String> {
        let creds = Credentials::new(smtp_config.username.clone(), smtp_config.password.clone());

        let mut builder =
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&smtp_config.host)
                .port(smtp_config.port)
                .credentials(creds);
        if smtp_config.tls {
            use lettre::transport::smtp::client::{Tls, TlsParameters};
            let params = TlsParameters::new(smtp_config.host.clone())
                .map_err(|e| format!("SMTP TLS setup failed: {e}"))?;
            builder = builder.tls(Tls::Required(params));
        }
        Ok(builder.build())
    }

    /// Sends the message, returning the SMTP response reference on
    /// success. Arrivals are viewed in the provider inbox (for Ethereal,
    /// log in at ethereal.email/messages with the account).
    ///
    /// The envelope From comes from the message itself (per-send sender
    /// identity); the provider config's `from_email` is only the fallback
    /// when the message carries none.
    pub async fn send_mail(&self, msg: &EmailMessage) -> Result<String, String> {
        let from_email = if msg.from_email.trim().is_empty() {
            self.config.from_address().to_string()
        } else {
            msg.from_email.clone()
        };
        let from: Address = from_email
            .parse()
            .map_err(|e| format!("Invalid from_address (want a bare email): {e}"))?;
        let from_name = msg
            .from_name
            .clone()
            .or_else(|| Self::from_name(&self.config));

        let mut b = Message::builder()
            .from(Mailbox::new(from_name, from))
            .subject(&msg.subject);

        for addr in &msg.to {
            let a: Address = addr
                .parse()
                .map_err(|e| format!("Invalid to '{addr}': {e}"))?;
            b = b.to(Mailbox::new(None, a));
        }

        for addr in &msg.cc {
            let a: Address = addr
                .parse()
                .map_err(|e| format!("Invalid cc '{addr}': {e}"))?;
            b = b.cc(Mailbox::new(None, a));
        }

        for addr in &msg.bcc {
            let a: Address = addr
                .parse()
                .map_err(|e| format!("Invalid bcc '{addr}': {e}"))?;
            b = b.bcc(Mailbox::new(None, a));
        }

        if let Some(reply_to) = &msg.reply_to {
            let a: Address = reply_to
                .parse()
                .map_err(|e| format!("Invalid reply_to '{reply_to}': {e}"))?;
            b = b.reply_to(Mailbox::new(None, a));
        }

        let email = if let Some(html) = &msg.body_html {
            b.multipart(
                MultiPart::alternative()
                    .singlepart(SinglePart::plain(msg.body_text.clone()))
                    .singlepart(SinglePart::html(html.clone())),
            )
            .map_err(|e| format!("Failed to build multipart email: {e}"))?
        } else {
            b.body(msg.body_text.clone())
                .map_err(|e| format!("Failed to build email: {e}"))?
        };

        // For SMTP config, use lettre directly. `tls` selects
        // STARTTLS upgrade (port 587 style); without it the
        // connection stays plaintext (local relays).
        match &self.config {
            EmailConfig::Smtp(smtp_config) => Self::smtp_transport(smtp_config)?
                .send(email)
                .await
                .map(|response| response.message().collect::<Vec<_>>().join(" "))
                .map_err(|e| format!("SMTP send failed: {e}")),
            _ => {
                // For API-based providers, use their specific HTTP clients
                // TODO: Implement SendGrid, Resend, etc. API calls
                Err(format!(
                    "Provider {} not yet implemented for direct sending",
                    self.config.provider_name()
                ))
            }
        }
    }
}
