pub mod config;
pub mod providers;

pub use config::EmailConfig;

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
    config: config::EmailConfig,
}

impl EmailSender {
    pub fn new(config: config::EmailConfig) -> Self {
        Self { config }
    }

    fn from_name(config: &config::EmailConfig) -> Option<String> {
        match config {
            config::EmailConfig::Smtp(c) => c.from_name.clone(),
            config::EmailConfig::SendGrid(c) => c.from_name.clone(),
            config::EmailConfig::Resend(c) => c.from_name.clone(),
            config::EmailConfig::AwsSes(c) => c.from_name.clone(),
            config::EmailConfig::Postmark(c) => c.from_name.clone(),
            config::EmailConfig::Mailgun(c) => c.from_name.clone(),
            config::EmailConfig::Brevo(c) => c.from_name.clone(),
        }
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
            config::EmailConfig::Smtp(smtp_config) => {
                let creds =
                    Credentials::new(smtp_config.username.clone(), smtp_config.password.clone());

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

                builder
                    .build()
                    .send(email)
                    .await
                    .map(|response| response.message().collect::<Vec<_>>().join(" "))
                    .map_err(|e| format!("SMTP send failed: {e}"))
            }
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
