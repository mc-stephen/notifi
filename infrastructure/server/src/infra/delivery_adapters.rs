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
use crate::channels::device_channel::providers::apple::apns::{
    ApnsMessage, ApnsProvider, IosConfig, MacosConfig,
};
use crate::channels::email_channel::{
    EmailMessage, EmailSender, config::EmailConfig, providers::smtp::SmtpConfig,
};
use crate::channels::sms_channel::providers::{
    africas_talking::AfricasTalkingConfig, local_nigeria::LocalNigeriaConfig, termii::TermiiConfig,
    twilio::TwilioConfig, vonage::VonageConfig,
};
use crate::channels::sms_channel::{SmsConfig, SmsMessage, SmsSender2};
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

/// Apple Push Notifications (channel `apple`): the APNs controller
/// signs with the project's `.p8` key and posts to Apple's gateway; the
/// adapter only maps the normalized message. One controller serves iOS
/// and macOS — the two provider ids (`apns`, `apns_macos`) pick the
/// bundle flavor from each one's saved config. The recipient is the raw
/// device token.
pub struct ApnsAdapter {
    provider: ApnsProvider,
    provider_id: &'static str,
}

impl ApnsAdapter {
    #[cfg(test)]
    fn with_provider(provider: ApnsProvider, provider_id: &'static str) -> Self {
        Self {
            provider,
            provider_id,
        }
    }
}

impl DeliveryProvider for ApnsAdapter {
    fn channel(&self) -> &'static str {
        "apple"
    }

    fn provider_name(&self) -> &'static str {
        self.provider_id
    }

    fn send(&self, message: &DeliveryMessage) -> BoxSendFuture<'_> {
        let provider = &self.provider;
        let provider_id = self.provider_id;
        let msg = ApnsMessage {
            device_token: message.recipient.clone(),
            title: message.title.clone(),
            body: message.body.clone(),
        };
        Box::pin(async move {
            provider
                .send_notification(&msg)
                .await
                .map(|_| DeliveryReceipt {
                    provider: provider_id.to_string(),
                    provider_message_id: None,
                })
                .map_err(DeliveryError::Provider)
        })
    }
}

/// Builds the APNs adapter for either OS flavor — same fields, different
/// bundle config; a missing/blank `.p8` path means "not wired".
fn apns_from_config(
    config: &serde_json::Value,
    provider_id: &str,
) -> Option<Arc<dyn DeliveryProvider>> {
    // The adapter's provider_name comes from the fixed id set.
    let provider_id: &'static str = if provider_id == "apns" {
        "apns"
    } else {
        "apns_macos"
    };
    let provider = if provider_id == "apns" {
        let cfg: IosConfig = serde_json::from_value(config.clone()).ok()?;
        if cfg.p8_key_path.as_os_str().is_empty() {
            return None;
        }
        ApnsProvider::new(cfg).ok()?
    } else {
        let cfg: MacosConfig = serde_json::from_value(config.clone()).ok()?;
        if cfg.p8_key_path.as_os_str().is_empty() {
            return None;
        }
        ApnsProvider::new_macos(cfg).ok()?
    };
    Some(Arc::new(ApnsAdapter {
        provider,
        provider_id,
    }))
}

/// SMS (channel `sms`): one adapter serves every wired vendor — the
/// family's `SmsSender2` dispatches to the vendor controller built from
/// the project's saved config. The recipient is the raw phone number;
/// SMS has no title concept, so only the body travels.
pub struct SmsAdapter {
    sender: SmsSender2,
    provider_id: &'static str,
}

impl DeliveryProvider for SmsAdapter {
    fn channel(&self) -> &'static str {
        "sms"
    }

    fn provider_name(&self) -> &'static str {
        self.provider_id
    }

    fn send(&self, message: &DeliveryMessage) -> BoxSendFuture<'_> {
        let sender = &self.sender;
        let provider_id = self.provider_id;
        let msg = SmsMessage {
            to: message.recipient.clone(),
            text: message.body.clone(),
        };
        Box::pin(async move {
            sender
                .send_sms(&msg)
                .await
                .map(|_| DeliveryReceipt {
                    provider: provider_id.to_string(),
                    provider_message_id: None,
                })
                .map_err(DeliveryError::Provider)
        })
    }
}

/// Builds the SMS adapter for one wired vendor: parse the vendor's bare
/// config JSON, wrap it in the tagged enum the family sender dispatches.
fn sms_adapter<T: serde::de::DeserializeOwned + 'static>(
    config: &serde_json::Value,
    provider_id: &'static str,
    variant: fn(T) -> crate::channels::sms_channel::SmsConfig,
) -> Option<Arc<dyn DeliveryProvider>> {
    let parsed: T = serde_json::from_value(config.clone()).ok()?;
    Some(Arc::new(SmsAdapter {
        sender: SmsSender2::new(variant(parsed)),
        provider_id,
    }))
}

/// Production factory: SMTP + Firebase + Apple + SMS from project config, stubs nowhere else yet.
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
            ("apple", "apns" | "apns_macos") => apns_from_config(config, provider_id),
            // The wired SMS vendors; the shells (bird, ebulksms,
            // infobip, smslive247) stay unwired until their controllers
            // exist.
            ("sms", "twilio") => sms_adapter::<TwilioConfig>(config, "twilio", SmsConfig::Twilio),
            ("sms", "termii") => sms_adapter::<TermiiConfig>(config, "termii", SmsConfig::Termii),
            ("sms", "africas_talking") => sms_adapter::<AfricasTalkingConfig>(
                config,
                "africas_talking",
                SmsConfig::AfricasTalking,
            ),
            ("sms", "local_nigeria") => {
                sms_adapter::<LocalNigeriaConfig>(config, "local_nigeria", SmsConfig::LocalNigeria)
            }
            ("sms", "vonage") => sms_adapter::<VonageConfig>(config, "vonage", SmsConfig::Vonage),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::ConfigProviderTester;
    use crate::ports::ProviderTester;
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

        // The apple arms arm from the same registry field set.
        assert!(
            f.adapter(
                "apple",
                "apns",
                &serde_json::json!({
                    "key_id": "K", "team_id": "T",
                    "bundle_id": "b", "p8_key_path": "/keys/authkey.p8"
                })
            )
            .is_some()
        );
        assert!(
            f.adapter(
                "apple",
                "apns_macos",
                &serde_json::json!({
                    "key_id": "K", "team_id": "T",
                    "bundle_id": "b", "p8_key_path": "/keys/authkey.p8"
                })
            )
            .is_some()
        );
        assert!(
            f.adapter(
                "apple",
                "apns",
                &serde_json::json!({
                    "key_id": "K", "team_id": "T", "bundle_id": "b", "p8_key_path": ""
                })
            )
            .is_none()
        );
        assert!(
            f.adapter("apple", "onesignal", &serde_json::json!({}))
                .is_none()
        );

        // The four wired SMS vendors arm; the five shells stay None.
        let twilio_cfg =
            serde_json::json!({"account_sid": "AC1", "auth_token": "t", "from_number": "+1"});
        let sms = f.adapter("sms", "twilio", &twilio_cfg);
        assert!(sms.is_some());
        let sms = sms.unwrap();
        assert_eq!(sms.channel(), "sms");
        assert_eq!(sms.provider_name(), "twilio");
        assert!(
            f.adapter(
                "sms",
                "termii",
                &serde_json::json!({"api_key": "k", "sender_id": "Notifi"})
            )
            .is_some()
        );
        assert!(
            f.adapter(
                "sms",
                "africas_talking",
                &serde_json::json!({"username": "u", "api_key": "k", "sender_id": "NOTIFI"})
            )
            .is_some()
        );
        assert!(
            f.adapter(
                "sms",
                "local_nigeria",
                &serde_json::json!({"api_key": "tok", "base_url": "https://x", "sender_id": "NOTIFI"})
            )
            .is_some()
        );
        // Shells stay unwired even with plausible config...
        assert!(
            f.adapter("sms", "infobip", &serde_json::json!({"api_key": "k"}))
                .is_none()
        );
        assert!(f.adapter("sms", "bird", &serde_json::json!({})).is_none());
        assert!(
            f.adapter("sms", "ebulksms", &serde_json::json!({}))
                .is_none()
        );
        assert!(
            f.adapter("sms", "smslive247", &serde_json::json!({}))
                .is_none()
        );
        // ...and a bad config for a wired vendor is None, never a panic.
        assert!(
            f.adapter("sms", "twilio", &serde_json::json!({"account_sid": "AC1"}))
                .is_none()
        );

        // Vonage arms (its probe already lived in the tester).
        let vonage = f.adapter(
            "sms",
            "vonage",
            &serde_json::json!({"api_key": "k", "api_secret": "s", "from_number": "Notifi"}),
        );
        assert!(vonage.is_some());
        assert_eq!(vonage.unwrap().provider_name(), "vonage");
    }

    /// Throwaway EC key for tests — signs test JWTs only, never a real
    /// credential.
    const TEST_P8_KEY: &str = "-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQg9mlu4yOHZr3ZV6W/
yENpstWvcb6awsO3MAoGba0Pn8uhRANCAASxuCOWXWK5YDSIdF73DthFO6YashYD
1QwErJ9yuwQuT4I4DAWWvbCAwVRzRhZdpOmRR6JPdhkuTwnLfl8d5ndU
-----END PRIVATE KEY-----";

    #[tokio::test]
    async fn apns_adapter_posts_the_normalized_message() {
        let stub = http_stub::start(200, "OK", "").await;
        let key_path =
            std::env::temp_dir().join(format!("notifi-apns-key-{}.p8", std::process::id()));
        std::fs::write(&key_path, TEST_P8_KEY).unwrap();
        let provider = ApnsProvider::new_with_base(
            IosConfig {
                key_id: "KEYID1".to_string(),
                team_id: "TEAMID9".to_string(),
                bundle_id: "app.notifi.test".to_string(),
                p8_key_path: key_path,
                sandbox: false,
            },
            format!("http://127.0.0.1:{}", stub.port),
        );
        let adapter = ApnsAdapter::with_provider(provider, "apns");

        let receipt = adapter
            .send(&DeliveryMessage {
                channel: "apple".to_string(),
                recipient: "device-token-9".to_string(),
                title: "Hi".to_string(),
                body: "Body".to_string(),
                metadata: None,
            })
            .await
            .expect("200 delivers");
        assert_eq!(receipt.provider, "apns");

        let raw = stub.request().await.expect("request captured");
        assert!(raw.contains("POST /3/device/device-token-9"), "path: {raw}");
        assert!(
            raw.to_ascii_lowercase()
                .contains("apns-topic: app.notifi.test"),
            "topic: {raw}"
        );
        assert!(raw.contains(r#""title":"Hi""#), "alert: {raw}");
        assert!(raw.contains(r#""body":"Body""#), "body: {raw}");
    }

    #[tokio::test]
    async fn apns_probe_signs_offline_and_rejects_broken_keys() {
        // A real EC key signs offline — no call to Apple.
        let key_path =
            std::env::temp_dir().join(format!("notifi-apns-probe-{}.p8", std::process::id()));
        std::fs::write(&key_path, TEST_P8_KEY).unwrap();
        let good = ConfigProviderTester
            .test(
                "apple",
                "apns",
                &serde_json::json!({
                    "key_id": "KEYID1", "team_id": "TEAMID9",
                    "bundle_id": "app.notifi.test", "p8_key_path": key_path.to_string_lossy()
                }),
            )
            .await;
        assert!(good.success, "{}", good.message);
        assert!(good.message.contains("signs"), "{}", good.message);

        // A missing key file fails the probe with Apple's reason.
        let bad = ConfigProviderTester
            .test(
                "apple",
                "apns",
                &serde_json::json!({
                    "key_id": "K", "team_id": "T",
                    "bundle_id": "b", "p8_key_path": "/nonexistent/authkey.p8"
                }),
            )
            .await;
        assert!(!bad.success, "{}", bad.message);
        assert!(bad.message.contains("Apple rejected"), "{}", bad.message);

        // The macOS flavor parses the same shape and passes with a key.
        let ok = ConfigProviderTester
            .test(
                "apple",
                "apns_macos",
                &serde_json::json!({
                    "key_id": "KEYID1", "team_id": "TEAMID9",
                    "bundle_id": "app.notifi.mac", "p8_key_path": key_path.to_string_lossy()
                }),
            )
            .await;
        assert!(ok.success, "{}", ok.message);
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
