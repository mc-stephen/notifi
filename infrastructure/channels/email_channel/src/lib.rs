mod config;

pub use config::EmailConfig;

use lettre::{
    Address, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
};

pub struct EmailMessage {
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub reply_to: Option<String>,
    pub subject: String,
    pub body_text: String,
    pub body_html: Option<String>,
}

pub struct EmailProvider {
    config: EmailConfig,
}

impl EmailProvider {
    pub fn new(config: EmailConfig) -> Self {
        Self { config }
    }

    pub async fn send_mail(&self, msg: &EmailMessage) -> Result<(), String> {
        let from: Address = self
            .config
            .from_address
            .parse()
            .map_err(|e| format!("Invalid from_address: {e}"))?;

        let mut b = Message::builder()
            .from(Mailbox::new(None, from))
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

        let creds = Credentials::new(self.config.username.clone(), self.config.password.clone());

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&self.config.host)
            .port(self.config.port)
            .credentials(creds)
            .build();

        mailer
            .send(email)
            .await
            .map_err(|e| format!("SMTP send failed: {e}"))?;

        Ok(())
    }
}
