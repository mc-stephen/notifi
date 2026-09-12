//! Live SMTP delivery for system mail, backed by `email_channel`.

use email_channel::{EmailMessage, EmailSender, config::EmailConfig, providers::smtp::SmtpConfig};

use crate::infra::config::MailConfig;
use crate::ports::mailer::{BoxFut, MailError, SmtpMailer, SystemSender};

pub struct LiveSmtpMailer {
    sender: EmailSender,
    mail_domain: String,
}

impl LiveSmtpMailer {
    /// `None` when SMTP is not configured (flows keep dev behavior).
    pub fn from_config(config: &MailConfig) -> Option<Self> {
        let host = config.smtp_host.clone().filter(|h| !h.trim().is_empty())?;
        Some(Self {
            sender: EmailSender::new(EmailConfig::Smtp(SmtpConfig {
                host,
                port: config.port_or_default(),
                username: config.smtp_username.clone().unwrap_or_default(),
                password: config.smtp_password.clone().unwrap_or_default(),
                // Transport fallback only — every send carries its own
                // resolved sender (see `SmtpMailer::send`).
                from_email: format!(
                    "no-reply@{}",
                    config.mail_domain.trim().trim_start_matches('@')
                ),
                from_name: Some("Notifi".to_string()),
                tls: config.smtp_tls,
            })),
            mail_domain: config.mail_domain.clone(),
        })
    }

    pub fn mail_domain(&self) -> &str {
        &self.mail_domain
    }
}

impl SmtpMailer for LiveSmtpMailer {
    fn send(
        &self,
        from: SystemSender,
        to: &str,
        subject: &str,
        text: &str,
        html: Option<&str>,
    ) -> BoxFut<'_, Result<String, MailError>> {
        let to = to.to_string();
        let subject = subject.to_string();
        let text = text.to_string();
        let html = html.map(str::to_string);
        let domain = self.mail_domain.clone();
        Box::pin(async move {
            let (from_email, from_name) = from.resolve(&domain)?;
            // Returns the SMTP response reference; arrivals are viewed
            // in the provider inbox (Ethereal: ethereal.email/messages).
            self.sender
                .send_mail(&EmailMessage {
                    from_email,
                    from_name,
                    to: vec![to],
                    cc: Vec::new(),
                    bcc: Vec::new(),
                    reply_to: None,
                    subject,
                    body_text: text,
                    body_html: html,
                })
                .await
                .map_err(MailError::Transport)
        })
    }
}
