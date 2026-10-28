//! Channel adapters for product sends (Phase A).
//!
//! Two vendors send for real today — SMTP (generic email) and Firebase
//! (android `fcm`) — built per-request from the project's own provider
//! config JSON. Every other channel/provider still resolves to `None` so
//! the send worker records a provider-missing failure and falls through
//! the priority chain — their adapters land as their controllers wire up.
//!
//! Adapters speak to ONE address (`DeliveryMessage.recipient` — an email
//! address, a push token); expanding a recipient's `devices` blob into
//! tokens is the send layer's job (§25).

use std::sync::Arc;

use crate::channels::device_channel::providers::android::{
    AndroidMessage,
    fcm::{FcmConfig, FcmProvider},
};
use crate::channels::email_channel::{
    EmailMessage, EmailSender, config::EmailConfig, providers::smtp::SmtpConfig,
};
use crate::ports::{
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

/// Firebase Cloud Messaging (android `fcm`): the vendor controller
/// speaks FCM HTTP v1 with a service-account credential; the adapter
/// only maps the normalized message onto the vendor's shape. The
/// recipient is the raw registration token.
pub struct FcmAdapter {
    provider: FcmProvider,
}

impl FcmAdapter {
    #[cfg(test)]
    fn with_provider(provider: FcmProvider) -> Self {
        Self { provider }
    }
}

impl DeliveryProvider for FcmAdapter {
    fn channel(&self) -> &'static str {
        "android"
    }

    fn provider_name(&self) -> &'static str {
        "fcm"
    }

    fn send(&self, message: &DeliveryMessage) -> BoxSendFuture<'_> {
        let provider = &self.provider;
        let msg = AndroidMessage {
            token: message.recipient.clone(),
            title: message.title.clone(),
            body: message.body.clone(),
        };
        Box::pin(async move {
            provider
                .send_notification(&msg)
                .await
                .map(|_| DeliveryReceipt {
                    provider: "fcm".to_string(),
                    provider_message_id: None,
                })
                .map_err(DeliveryError::Provider)
        })
    }
}

/// Production factory: SMTP + Firebase from project config, stubs nowhere else yet.
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
            ("android", "fcm") => {
                let cfg: FcmConfig = serde_json::from_value(config.clone()).ok()?;
                if cfg.service_account_path.as_os_str().is_empty() {
                    return None;
                }
                Some(Arc::new(FcmAdapter {
                    provider: FcmProvider::new(cfg),
                }))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::http_stub;

    fn message() -> DeliveryMessage {
        DeliveryMessage {
            channel: "android".to_string(),
            recipient: "device-token-1".to_string(),
            title: "Hi".to_string(),
            body: "Body".to_string(),
            metadata: None,
        }
    }

    #[test]
    fn factory_arms_email_and_fcm_only() {
        let f = DefaultAdapterFactory;

        // The email arm keeps its full-field parse (username/password required).
        assert!(
            f.adapter(
                "email",
                "smtp",
                &serde_json::json!({
                    "host": "smtp.x.com", "port": 587,
                    "username": "u", "password": "p",
                    "from_email": "a@b.c"
                })
            )
            .is_some()
        );
        assert!(
            f.adapter("email", "smtp", &serde_json::json!({"host": ""}))
                .is_none()
        );

        // The fcm arm parses its registry field (`service_account_path`).
        let fcm = f.adapter(
            "android",
            "fcm",
            &serde_json::json!({"service_account_path": "/keys/sa.json"}),
        );
        assert!(fcm.is_some());
        let fcm = fcm.unwrap();
        assert_eq!(fcm.channel(), "android");
        assert_eq!(fcm.provider_name(), "fcm");

        // Missing / blank credential config resolves to None, and every
        // not-yet-wired pair stays None (never a panic).
        assert!(
            f.adapter("android", "fcm", &serde_json::json!({}))
                .is_none()
        );
        assert!(
            f.adapter(
                "android",
                "fcm",
                &serde_json::json!({"service_account_path": ""})
            )
            .is_none()
        );
        assert!(
            f.adapter("android", "pushy", &serde_json::json!({}))
                .is_none()
        );
        assert!(f.adapter("sms", "twilio", &serde_json::json!({})).is_none());
    }

    #[tokio::test]
    async fn fcm_adapter_posts_the_normalized_message() {
        let stub = http_stub::start(200, "OK", r#"{"name":"m1"}"#).await;
        // The SA file only needs project_id — the canned token override
        // (from new_with_fcm_base) skips Google's OAuth service entirely.
        let sa = std::env::temp_dir().join(format!("notifi-fcm-sa-{}.json", std::process::id()));
        std::fs::write(&sa, r#"{"project_id": "proj-1"}"#).unwrap();
        let provider = FcmProvider::new_with_fcm_base(
            FcmConfig {
                service_account_path: sa,
            },
            format!("http://127.0.0.1:{}", stub.port),
        );
        let adapter = FcmAdapter::with_provider(provider);

        let receipt = adapter.send(&message()).await.expect("200 delivers");
        assert_eq!(receipt.provider, "fcm");

        let raw = stub.request().await.expect("request captured");
        assert!(
            raw.contains("POST /v1/projects/proj-1/messages:send"),
            "path: {raw}"
        );
        assert!(raw.contains(r#""token":"device-token-1""#), "token: {raw}");
        assert!(raw.contains(r#""title":"Hi""#), "title: {raw}");
        assert!(raw.contains(r#""body":"Body""#), "body: {raw}");
    }
}
