//! Channel adapters for product sends (Phase A).
//!
//! Only SMTP (generic email) sends for real today, built per-request from
//! the project's own provider config JSON. Every other channel/provider
//! resolves to `None` so the send worker records a provider-missing
//! failure and falls through the priority chain — adapters land with M3.

use std::sync::Arc;

use email_channel::{EmailMessage, EmailSender, config::EmailConfig, providers::smtp::SmtpConfig};
use notifi_domain_ports::{
    AdapterFactory, BoxSendFuture, DeliveryError, DeliveryMessage, DeliveryProvider,
    DeliveryReceipt,
};

pub struct SmtpAdapter {
    sender: EmailSender,
    from_email: String,
    from_name: Option<String>,
}

impl DeliveryProvider for SmtpAdapter {
    fn channel(&self) -> &'static str {
        "email"
    }

    fn provider_name(&self) -> &'static str {
        "smtp"
    }

    fn send(&self, message: &DeliveryMessage) -> BoxSendFuture<'_> {
        let sender = &self.sender;
        let msg = EmailMessage {
            from_email: self.from_email.clone(),
            from_name: self.from_name.clone(),
            to: vec![message.recipient.clone()],
            cc: Vec::new(),
            bcc: Vec::new(),
            reply_to: None,
            subject: message.title.clone(),
            body_text: message.body.clone(),
            body_html: None,
        };
        Box::pin(async move {
            sender
                .send_mail(&msg)
                .await
                .map(|reference| DeliveryReceipt {
                    provider: "smtp".to_string(),
                    provider_message_id: Some(reference),
                })
                .map_err(DeliveryError::Provider)
        })
    }
}

/// Production factory: SMTP from project config, stubs nowhere else yet.
pub struct DefaultAdapterFactory;

impl AdapterFactory for DefaultAdapterFactory {
    fn adapter(
        &self,
        channel: &str,
        provider_id: &str,
        config: &serde_json::Value,
    ) -> Option<Arc<dyn DeliveryProvider>> {
        match (channel, provider_id) {
            ("email", "smtp") => {
                let cfg: SmtpConfig = serde_json::from_value(config.clone()).ok()?;
                if cfg.host.trim().is_empty() {
                    return None;
                }
                let from_email = cfg.from_email.clone();
                let from_name = cfg.from_name.clone();
                let sender = EmailSender::new(EmailConfig::Smtp(cfg));
                Some(Arc::new(SmtpAdapter {
                    sender,
                    from_email,
                    from_name,
                }))
            }
            _ => None,
        }
    }
}
