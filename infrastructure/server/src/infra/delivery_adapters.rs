//! Channel adapters for product sends (Phase A).
//!
//! Email (SMTP), android (fcm / pushy / onesignal), apple (apns /
//! onesignal), window (WNS),
//! web push, the whole chat family (slack /
//! telegram / whatsapp_business / discord / ms_teams), all nine
//! SMS vendors and the other channel (onesignal wrapping both phone
//! families at once, pushy wrapping phone families plus web / webhook
//! relay / the Notifi gateway pull-route) send for real — one adapter set
//! serves every destination in the registry. Each adapter is built
//! per-request from the project's own provider config JSON. Anything
//! else still resolves to `None` so the send worker records a
//! provider-missing failure and falls through the priority chain —
//! their adapters land as their controllers wire up.
//!
//! Adapters speak to ONE address (`DeliveryMessage.recipient` — an email
//! address, a push token, a raw phone number); expanding a recipient's
//! `devices` blob into tokens is the send layer's job (§25).

use std::sync::Arc;

use crate::channels::chat_channel::{ChatMessage, ChatSender2, config::ChatConfig};
use crate::channels::device_channel::android::{
    AndroidMessage,
    fcm::{FcmConfig, FcmProvider},
};
use crate::channels::device_channel::apple::apns::{ApnsMessage, ApnsProvider, IosConfig};
use crate::channels::device_channel::web::web_push::{WebConfig, WebMessage, WebProvider};
use crate::channels::device_channel::window::{
    WindowSender,
    wns::{WnsConfig, WnsProvider},
};
use crate::channels::email_channel::{
    EmailMessage, EmailSender, config::EmailConfig, smtp::SmtpConfig,
};
use crate::channels::other_channel::{
    OtherChannelConfig,
    onesignal::{OneSignalConfig, OneSignalMessage, OneSignalProvider},
    pushy::{PushyConfig, PushyMessage, PushyProvider},
    relay::RelayProvider,
};
use crate::channels::sms_channel::{SmsConfig, SmsMessage, SmsSender2};
use crate::channels::sms_channel::{
    africas_talking::AfricasTalkingConfig, bird::BirdConfig, ebulksms::EbulkSmsConfig,
    infobip::InfobipConfig, local_nigeria::LocalNigeriaConfig, smslive247::SmsLive247Config,
    termii::TermiiConfig, twilio::TwilioConfig, vonage::VonageConfig,
};
use crate::ports::gateway::GatewayStore;
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
/// adapter only maps the normalized message. One recipe covers the
/// app's Apple installs — Apple routes each token to its device. The
/// recipient is the raw device token.
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

/// Builds the APNs adapter from the saved recipe — a missing/blank
/// `.p8` path means "not wired".
fn apns_from_config(config: &serde_json::Value) -> Option<Arc<dyn DeliveryProvider>> {
    let cfg: IosConfig = serde_json::from_value(config.clone()).ok()?;
    if cfg.p8_key_path.as_os_str().is_empty() {
        return None;
    }
    Some(Arc::new(ApnsAdapter {
        provider: ApnsProvider::new(cfg).ok()?,
        provider_id: "apns",
    }))
}

/// Pushy: posts the notification to Pushy's push endpoint; the
/// recipient is the device registration token. Same shape as the
/// Firebase adapter — one vendor, one controller file, the warhead is
/// the config. The card lives in the other channel's section (Pushy
/// serves android, apple and web from one key), and legacy android
/// saves keep delivering through their own arm.
pub struct PushyAdapter {
    provider: PushyProvider,
    channel: &'static str,
}

impl DeliveryProvider for PushyAdapter {
    fn channel(&self) -> &'static str {
        self.channel
    }

    fn provider_name(&self) -> &'static str {
        "pushy"
    }

    fn send(&self, message: &DeliveryMessage) -> BoxSendFuture<'_> {
        let provider = &self.provider;
        let msg = PushyMessage {
            token: message.recipient.clone(),
            title: message.title.clone(),
            body: message.body.clone(),
        };
        Box::pin(async move {
            provider
                .send_notification(&msg)
                .await
                .map(|_| DeliveryReceipt {
                    provider: "pushy".to_string(),
                    provider_message_id: None,
                })
                .map_err(DeliveryError::Provider)
        })
    }
}

/// OneSignal (channels `android` + `apple`): one REST API genuinely
/// serves both platforms, so one adapter is armed for both pairs. The
/// recipient is the subscription id; auth is the REST API key the
/// config prefers (falling back to the plain api key).
pub struct OneSignalAdapter {
    provider: OneSignalProvider,
    channel: &'static str,
}

impl DeliveryProvider for OneSignalAdapter {
    fn channel(&self) -> &'static str {
        self.channel
    }

    fn provider_name(&self) -> &'static str {
        "onesignal"
    }

    fn send(&self, message: &DeliveryMessage) -> BoxSendFuture<'_> {
        let provider = &self.provider;
        let msg = OneSignalMessage {
            token: message.recipient.clone(),
            title: message.title.clone(),
            body: message.body.clone(),
        };
        Box::pin(async move {
            provider
                .send_notification(&msg)
                .await
                .map(|_| DeliveryReceipt {
                    provider: "onesignal".to_string(),
                    provider_message_id: None,
                })
                .map_err(DeliveryError::Provider)
        })
    }
}

/// Builds the shared OneSignal adapter for either destination.
fn onesignal_adapter(
    config: &serde_json::Value,
    channel: &'static str,
) -> Option<Arc<dyn DeliveryProvider>> {
    // Extracted by hand, not via the vendor struct: the REST key or the
    // plain api key alone must both arm — the dashboard may save either
    // field first.
    let obj = config.as_object()?;
    let app_id = obj.get("app_id").and_then(|v| v.as_str()).unwrap_or("");
    let api_key = obj.get("api_key").and_then(|v| v.as_str()).unwrap_or("");
    let rest = obj
        .get("rest_api_key")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if app_id.trim().is_empty() {
        return None;
    }
    let rest_key = if rest.trim().is_empty() {
        None
    } else {
        Some(rest.to_string())
    };
    if rest_key.is_none() && api_key.trim().is_empty() {
        return None;
    }
    Some(Arc::new(OneSignalAdapter {
        provider: OneSignalProvider::new(OneSignalConfig {
            app_id: app_id.to_string(),
            api_key: api_key.to_string(),
            rest_api_key: rest_key,
        }),
        channel,
    }))
}

/// Web push (channel `web`): the recipient is the browser's whole
/// subscription — web push needs endpoint + client keys together, so
/// the one address string is the `PushSubscription.toJSON()` payload
/// `{"endpoint":..., "keys":{"p256dh":..., "auth":...}}`. The payload
/// is the body alone (web push carries no separate title).
pub struct WebPushAdapter {
    provider: WebProvider,
}

#[derive(serde::Deserialize)]
struct WebSubscription {
    endpoint: String,
    keys: WebSubscriptionKeys,
}

#[derive(serde::Deserialize)]
struct WebSubscriptionKeys {
    p256dh: String,
    auth: String,
}

impl DeliveryProvider for WebPushAdapter {
    fn channel(&self) -> &'static str {
        "web"
    }

    fn provider_name(&self) -> &'static str {
        "web_push"
    }

    fn send(&self, message: &DeliveryMessage) -> BoxSendFuture<'_> {
        let provider = &self.provider;
        let Ok(sub) = serde_json::from_str::<WebSubscription>(&message.recipient) else {
            return Box::pin(async {
                Err(DeliveryError::Provider(
                    "web push recipient must be the browser subscription json \
                     (PushSubscription.toJSON shape)"
                        .to_string(),
                ))
            });
        };
        let msg = WebMessage {
            endpoint: sub.endpoint,
            p256dh: sub.keys.p256dh,
            auth: sub.keys.auth,
            text: message.body.clone(),
        };
        Box::pin(async move {
            provider
                .send_notification(&msg)
                .await
                .map(|_| DeliveryReceipt {
                    provider: "web_push".to_string(),
                    provider_message_id: None,
                })
                .map_err(DeliveryError::Provider)
        })
    }
}

/// Window (channel `window`): one adapter serves both flavors (webhook
/// and WNS) — the flavor's Sender is built from the project's saved
/// config and holds the targeting. The recipient is the target address
/// (webhook registration URL or WNS channel URI).
pub struct WindowAdapter {
    sender: Box<dyn WindowSender>,
    provider_id: &'static str,
}

impl DeliveryProvider for WindowAdapter {
    fn channel(&self) -> &'static str {
        "window"
    }

    fn provider_name(&self) -> &'static str {
        self.provider_id
    }

    fn send(&self, message: &DeliveryMessage) -> BoxSendFuture<'_> {
        let sender = &self.sender;
        let provider_id = self.provider_id;
        let target = message.recipient.clone();
        let title = message.title.clone();
        let body = message.body.clone();
        Box::pin(async move {
            sender
                .send(&target, &title, &body)
                .await
                .map(|_| DeliveryReceipt {
                    provider: provider_id.to_string(),
                    provider_message_id: None,
                })
                .map_err(DeliveryError::Provider)
        })
    }
}

/// Chat (channel `chat`): one adapter serves every chat vendor. The
/// saved provider config is the tagged envelope the channel's own
/// config module owns — provider name first, vendor fields after — so
/// the vendor split happens inside ChatConfig and this adapter only
/// routes. The recipient is the per-send chat address: whatsapp reads
/// it as the to-number; the channel-bound flavors (slack/telegram/
/// discord bots, each webhook) target the channel saved in the
/// provider config. Titles travel as the headline (each controller
/// renders it its platform's way) and the body as the text.
pub struct ChatAdapter {
    sender: ChatSender2,
    provider_id: &'static str,
}

impl DeliveryProvider for ChatAdapter {
    fn channel(&self) -> &'static str {
        "chat"
    }

    fn provider_name(&self) -> &'static str {
        self.provider_id
    }

    fn send(&self, message: &DeliveryMessage) -> BoxSendFuture<'_> {
        let sender = &self.sender;
        let provider_id = self.provider_id;
        let msg = ChatMessage {
            channel_id: Some(message.recipient.clone()),
            title: Some(message.title.clone()),
            text: message.body.clone(),
            metadata: None,
        };
        Box::pin(async move {
            sender
                .send_message(&msg)
                .await
                .map(|_| DeliveryReceipt {
                    provider: provider_id.to_string(),
                    provider_message_id: None,
                })
                .map_err(DeliveryError::Provider)
        })
    }
}

/// Builds the chat adapter from the saved envelope. The tag must agree
/// with the saved provider id (both come from the same registry card;
/// a disagreement is a miswiring, not a sendable state), and the
/// vendor tag maps to its known static name.
fn chat_adapter(
    config: &serde_json::Value,
    provider_id: &str,
) -> Option<Arc<dyn DeliveryProvider>> {
    let tag = config
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let name: &'static str = match tag {
        "slack" => "slack",
        "telegram" => "telegram",
        "whatsapp_business" => "whatsapp_business",
        "discord" => "discord",
        "ms_teams" => "ms_teams",
        _ => return None,
    };
    if provider_id != name {
        return None;
    }
    let cfg: ChatConfig = serde_json::from_value(config.clone()).ok()?;
    if !cfg.carries_credentials() {
        return None;
    }
    Some(Arc::new(ChatAdapter {
        sender: ChatSender2::new(cfg),
        provider_id: name,
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
/// The Notifi gateway (channel `other`, provider `gateway`): queues the
/// notification into Notifi's own gateway queue — the user's app polls
/// its secret route (`GET /gateway/{topic}?key=...`) and reads it away.
/// No external provider at all: the store IS the vendor's listening
/// path (see `ports::gateway`). The recipe carries the route's two
/// halves, so the recipient contact is unused here.
pub struct GatewayAdapter {
    store: Arc<dyn GatewayStore + Send + Sync>,
    topic: String,
    secret_key: String,
}

impl DeliveryProvider for GatewayAdapter {
    fn channel(&self) -> &'static str {
        "other"
    }

    fn provider_name(&self) -> &'static str {
        "gateway"
    }

    fn send(&self, message: &DeliveryMessage) -> BoxSendFuture<'_> {
        let store = self.store.clone();
        let topic = self.topic.clone();
        let secret_key = self.secret_key.clone();
        let title = message.title.clone();
        let body = message.body.clone();
        Box::pin(async move {
            store
                .push(&topic, &secret_key, &title, &body)
                .await
                .map(|_| DeliveryReceipt {
                    provider: "gateway".to_string(),
                    provider_message_id: None,
                })
                .map_err(|e| DeliveryError::Provider(e.to_string()))
        })
    }
}

/// Builds channel adapters from per-project provider config.
///
/// Most arms are self-contained (the recipe carries the credential); the
/// gateway arm is the exception — its send writes into Notifi's own
/// device queue, so the factory holds the store when the server is wired
/// with a database (unit-defaulted factories arm everything but it).
#[derive(Default)]
pub struct DefaultAdapterFactory {
    /// Device gateway queue (`ports::gateway`) — the `("other","gateway")`
    /// arm's write target (`None` in store-less deployments/tests).
    gateway: Option<Arc<dyn GatewayStore + Send + Sync>>,
}

impl DefaultAdapterFactory {
    /// Factory wired with the server's gateway queue.
    pub fn new(gateway: Arc<dyn GatewayStore + Send + Sync>) -> Self {
        Self {
            gateway: Some(gateway),
        }
    }
}

/// Webhook relay (channel `other`): posts the rendered payload to the
/// target URL saved at connect time — the recipe carries url, method,
/// headers and the payload template, so the per-send address is
/// informational only. The relay controller does the HTTP work; this
/// adapter is the other channel's arm around it.
pub struct WebhookRelayAdapter {
    provider: RelayProvider,
    project: String,
}

impl DeliveryProvider for WebhookRelayAdapter {
    fn channel(&self) -> &'static str {
        "other"
    }

    fn provider_name(&self) -> &'static str {
        "relay"
    }

    fn send(&self, message: &DeliveryMessage) -> BoxSendFuture<'_> {
        let provider = &self.provider;
        let title = message.title.clone();
        let body = message.body.clone();
        let project = self.project.clone();
        Box::pin(async move {
            provider
                .send(&title, &body, &project)
                .await
                .map(|_| DeliveryReceipt {
                    provider: "relay".to_string(),
                    provider_message_id: None,
                })
                .map_err(DeliveryError::Provider)
        })
    }
}

impl AdapterFactory for DefaultAdapterFactory {
    fn adapter(
        &self,
        project_id: &str,
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
            ("android", "pushy") => {
                let cfg: PushyConfig = serde_json::from_value(config.clone()).ok()?;
                if cfg.api_key.trim().is_empty() {
                    return None;
                }
                Some(Arc::new(PushyAdapter {
                    provider: PushyProvider::new(cfg),
                    channel: "android",
                }))
            }
            ("android", "onesignal") => onesignal_adapter(config, "android"),
            ("apple", "onesignal") => onesignal_adapter(config, "apple"),
            ("apple", "apns") => apns_from_config(config),
            ("window", "wns") => {
                let cfg: WnsConfig = serde_json::from_value(config.clone()).ok()?;
                if cfg.client_id.trim().is_empty() || cfg.client_secret.trim().is_empty() {
                    return None;
                }
                Some(Arc::new(WindowAdapter {
                    sender: Box::new(WnsProvider::new(
                        cfg.client_id,
                        cfg.client_secret,
                        cfg.package_sid,
                    )),
                    provider_id: "wns",
                }))
            }
            ("web", "web_push") => {
                let cfg: WebConfig = serde_json::from_value(config.clone()).ok()?;
                if cfg.vapid_public_key.trim().is_empty()
                    || cfg.vapid_private_key.trim().is_empty()
                    || cfg.contact_email.trim().is_empty()
                {
                    return None;
                }
                Some(Arc::new(WebPushAdapter {
                    provider: WebProvider::new(cfg).ok()?,
                }))
            }
            ("chat", provider_id) => chat_adapter(config, provider_id),
            ("other", "onesignal") => {
                let parsed: OtherChannelConfig = serde_json::from_value(config.clone()).ok()?;
                if !parsed.carries_credentials() {
                    return None;
                }
                let OtherChannelConfig::OneSignal {
                    app_id,
                    api_key,
                    rest_api_key,
                } = parsed
                else {
                    return None;
                };
                Some(Arc::new(OneSignalAdapter {
                    provider: OneSignalProvider::new(OneSignalConfig {
                        app_id,
                        api_key,
                        rest_api_key,
                    }),
                    channel: "other",
                }))
            }
            ("other", "pushy") => {
                let parsed: OtherChannelConfig = serde_json::from_value(config.clone()).ok()?;
                if !parsed.carries_credentials() {
                    return None;
                }
                let OtherChannelConfig::Pushy { api_key, app_id } = parsed else {
                    return None;
                };
                Some(Arc::new(PushyAdapter {
                    provider: PushyProvider::new(PushyConfig { api_key, app_id }),
                    channel: "other",
                }))
            }
            ("other", "relay") => {
                let parsed: OtherChannelConfig = serde_json::from_value(config.clone()).ok()?;
                if !parsed.carries_credentials() {
                    return None;
                }
                let OtherChannelConfig::Relay {
                    target_url,
                    method,
                    headers,
                    payload_template,
                } = parsed
                else {
                    return None;
                };
                Some(Arc::new(WebhookRelayAdapter {
                    provider: RelayProvider::new(target_url, method, headers, payload_template),
                    project: project_id.to_string(),
                }))
            }
            ("other", "gateway") => {
                let parsed: OtherChannelConfig = serde_json::from_value(config.clone()).ok()?;
                if !parsed.carries_credentials() {
                    return None;
                }
                let OtherChannelConfig::Gateway {
                    topic, secret_key, ..
                } = parsed
                else {
                    return None;
                };
                let store = self.gateway.clone()?;
                Some(Arc::new(GatewayAdapter {
                    store,
                    topic,
                    secret_key,
                }))
            }
            // All nine SMS vendors arm from their own bare saved config.
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
            ("sms", "infobip") => {
                sms_adapter::<InfobipConfig>(config, "infobip", SmsConfig::Infobip)
            }
            ("sms", "bird") => sms_adapter::<BirdConfig>(config, "bird", SmsConfig::Bird),
            ("sms", "ebulksms") => {
                sms_adapter::<EbulkSmsConfig>(config, "ebulksms", SmsConfig::EbulkSms)
            }
            ("sms", "smslive247") => {
                sms_adapter::<SmsLive247Config>(config, "smslive247", SmsConfig::SmsLive247)
            }
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

    /// Gateway-arm spied routes: (topic, key, title, body) per push.
    static GATEWAY_LOG: std::sync::Mutex<Vec<(String, String, String, String)>> =
        std::sync::Mutex::new(Vec::new());

    /// In-file gateway queue fake — records pushes in GATEWAY_LOG; poll
    /// stays memoryless (factory tests only prove the write-through).
    struct RecordingGatewayStore;

    impl GatewayStore for RecordingGatewayStore {
        fn push(
            &self,
            topic: &str,
            key: &str,
            title: &str,
            body: &str,
        ) -> crate::ports::gateway::BoxGatewayFuture<'_, ()> {
            GATEWAY_LOG.lock().unwrap().push((
                topic.to_string(),
                key.to_string(),
                title.to_string(),
                body.to_string(),
            ));
            Box::pin(async { Ok(()) })
        }

        fn poll(
            &self,
            _topic: &str,
            _key: &str,
            _wait_secs: u64,
        ) -> crate::ports::gateway::BoxGatewayFuture<'_, Vec<crate::ports::gateway::GatewayMessage>>
        {
            Box::pin(async { Ok(Vec::new()) })
        }
    }

    /// Factory wired with the recording store (fresh log each call).
    fn factory_with_store() -> DefaultAdapterFactory {
        GATEWAY_LOG.lock().unwrap().clear();
        DefaultAdapterFactory::new(Arc::new(RecordingGatewayStore))
    }

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
        let f = DefaultAdapterFactory::default();

        // The email arm keeps its full-field parse (username/password required).
        assert!(
            f.adapter(
                "proj-1",
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
            f.adapter("proj-1", "email", "smtp", &serde_json::json!({"host": ""}))
                .is_none()
        );

        // The fcm arm parses its registry field (`service_account_path`).
        let fcm = f.adapter(
            "proj-1",
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
            f.adapter("proj-1", "android", "fcm", &serde_json::json!({}))
                .is_none()
        );
        assert!(
            f.adapter(
                "proj-1",
                "android",
                "fcm",
                &serde_json::json!({"service_account_path": ""})
            )
            .is_none()
        );
        assert!(
            f.adapter("proj-1", "android", "pushy", &serde_json::json!({}))
                .is_none()
        );
        // The pushy arm carries an api key.
        let pushy = f.adapter(
            "proj-1",
            "android",
            "pushy",
            &serde_json::json!({"api_key": "k"}),
        );
        assert!(pushy.is_some());
        let pushy = pushy.unwrap();
        assert_eq!(pushy.channel(), "android");
        assert_eq!(pushy.provider_name(), "pushy");
        assert!(
            f.adapter("proj-1", "sms", "twilio", &serde_json::json!({}))
                .is_none()
        );

        // The apple arms arm from the same registry field set.
        assert!(
            f.adapter(
                "proj-1",
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
                "proj-1",
                "apple",
                "apns",
                &serde_json::json!({
                    "key_id": "K", "team_id": "T", "bundle_id": "b", "p8_key_path": ""
                })
            )
            .is_none()
        );
        // OneSignal arms for BOTH destinations (one REST api serves
        // both platforms); blanks stay None.
        let onesignal = f.adapter(
            "proj-1",
            "android",
            "onesignal",
            &serde_json::json!({"app_id": "appid", "api_key": "k"}),
        );
        assert!(onesignal.is_some());
        assert_eq!(onesignal.unwrap().provider_name(), "onesignal");
        let onesignal = f.adapter(
            "proj-1",
            "apple",
            "onesignal",
            &serde_json::json!({"app_id": "appid", "rest_api_key": "k"}),
        );
        assert!(onesignal.is_some());
        assert_eq!(onesignal.unwrap().channel(), "apple");
        assert!(
            f.adapter(
                "proj-1",
                "android",
                "onesignal",
                &serde_json::json!({"app_id": "appid"})
            )
            .is_none()
        );

        // The WNS flavor arms from its credential pair; a blank one stays None.
        let wns = f.adapter(
            "proj-1",
            "window",
            "wns",
            &serde_json::json!({
                "client_id": "cid", "client_secret": "sec", "package_sid": "sid"
            }),
        );
        assert!(wns.is_some());
        let wns = wns.unwrap();
        assert_eq!(wns.channel(), "window");
        assert_eq!(wns.provider_name(), "wns");
        assert!(
            f.adapter(
                "proj-1",
                "window",
                "wns",
                &serde_json::json!({"client_id": "", "client_secret": "sec"})
            )
            .is_none()
        );

        // The web push arm arms from the VAPID triple; blanks stay None.
        let web = f.adapter(
            "proj-1",
            "web",
            "web_push",
            &serde_json::json!({
                "vapid_public_key": "pub",
                "vapid_private_key": "-----BEGIN PRIVATE KEY-----",
                "contact_email": "ops@notifi.dev"
            }),
        );
        assert!(web.is_some());
        let web = web.unwrap();
        assert_eq!(web.channel(), "web");
        assert_eq!(web.provider_name(), "web_push");
        assert!(
            f.adapter(
                "proj-1",
                "web",
                "web_push",
                &serde_json::json!({
                    "vapid_public_key": "pub",
                    "vapid_private_key": "",
                    "contact_email": "ops@notifi.dev"
                })
            )
            .is_none()
        );

        // The chat arm parses the tagged envelope its config module
        // owns — the vendor split happens inside ChatConfig.
        let chat = f.adapter(
            "proj-1",
            "chat",
            "slack",
            &serde_json::json!({
                "provider": "slack", "type": "webhook",
                "webhook_url": "https://hooks.slack.com/x"
            }),
        );
        assert!(chat.is_some());
        let chat = chat.unwrap();
        assert_eq!(chat.channel(), "chat");
        assert_eq!(chat.provider_name(), "slack");
        assert!(
            f.adapter(
                "proj-1",
                "chat",
                "telegram",
                &serde_json::json!({"provider": "telegram", "bot_token": "b", "chat_id": "c"})
            )
            .is_some()
        );
        assert!(
            f.adapter(
                "proj-1",
                "chat",
                "whatsapp_business",
                &serde_json::json!({
                    "provider": "whatsapp_business",
                    "phone_number_id": "p", "access_token": "a"
                })
            )
            .is_some()
        );
        assert!(
            f.adapter(
                "proj-1",
                "chat",
                "discord",
                &serde_json::json!({
                    "provider": "discord", "type": "bot",
                    "bot_token": "b", "channel_id": "c"
                })
            )
            .is_some()
        );
        assert!(
            f.adapter(
                "proj-1",
                "chat",
                "ms_teams",
                &serde_json::json!({
                    "provider": "ms_teams",
                    "webhook_url": "https://outlook.office.com/webhook/x"
                })
            )
            .is_some()
        );
        assert!(
            f.adapter(
                "proj-1",
                "chat",
                "telegram",
                &serde_json::json!({"provider": "telegram", "bot_token": "", "chat_id": "c"})
            )
            .is_none()
        );
        // The saved id must agree with the envelope tag.
        assert!(
            f.adapter(
                "proj-1",
                "chat",
                "slack",
                &serde_json::json!({"provider": "telegram", "bot_token": "b", "chat_id": "c"})
            )
            .is_none()
        );

        // The wired SMS vendors arm...
        let twilio_cfg =
            serde_json::json!({"account_sid": "AC1", "auth_token": "t", "from_number": "+1"});
        let sms = f.adapter("proj-1", "sms", "twilio", &twilio_cfg);
        assert!(sms.is_some());
        let sms = sms.unwrap();
        assert_eq!(sms.channel(), "sms");
        assert_eq!(sms.provider_name(), "twilio");
        assert!(
            f.adapter(
                "proj-1",
                "sms",
                "termii",
                &serde_json::json!({"api_key": "k", "sender_id": "Notifi"})
            )
            .is_some()
        );
        assert!(
            f.adapter(
                "proj-1",
                "sms",
                "africas_talking",
                &serde_json::json!({"username": "u", "api_key": "k", "sender_id": "NOTIFI"})
            )
            .is_some()
        );
        assert!(
            f.adapter("proj-1", 
                "sms",
                "local_nigeria",
                &serde_json::json!({"api_key": "tok", "base_url": "https://x", "sender_id": "NOTIFI"})
            )
            .is_some()
        );
        // A bad config for a wired vendor is None, never a panic.
        assert!(
            f.adapter(
                "proj-1",
                "sms",
                "twilio",
                &serde_json::json!({"account_sid": "AC1"})
            )
            .is_none()
        );
        // An unknown sms vendor is unwired, never a panic.
        assert!(
            f.adapter("proj-1", "sms", "who_dis", &serde_json::json!({}))
                .is_none()
        );
        // Bird arms.
        let bird = f.adapter(
            "proj-1",
            "sms",
            "bird",
            &serde_json::json!({
                "api_key": "k",
                "base_url": "https://rest.messagebird.com",
                "sender_id": "Notifi"
            }),
        );
        assert!(bird.is_some());
        assert_eq!(bird.unwrap().provider_name(), "bird");

        // EbulkSms arms (auth is the username+apikey pair).
        let ebulk = f.adapter(
            "proj-1",
            "sms",
            "ebulksms",
            &serde_json::json!({
                "username": "a@b.c", "api_key": "k", "sender_id": "Notifi"
            }),
        );
        assert!(ebulk.is_some());
        assert_eq!(ebulk.unwrap().provider_name(), "ebulksms");

        // SmsLive247 arms (the last of the nine).
        let live = f.adapter(
            "proj-1",
            "sms",
            "smslive247",
            &serde_json::json!({"api_key": "k", "sender_id": "Notifi"}),
        );
        assert!(live.is_some());
        assert_eq!(live.unwrap().provider_name(), "smslive247");

        // Infobip arms (probe already lived in the tester).
        let infobip = f.adapter(
            "proj-1",
            "sms",
            "infobip",
            &serde_json::json!({
                "api_key": "k",
                "base_url": "https://acct.api.infobip.com",
                "sender_id": "Notifi"
            }),
        );
        assert!(infobip.is_some());
        assert_eq!(infobip.unwrap().provider_name(), "infobip");

        // Vonage arms (its probe already lived in the tester).
        let vonage = f.adapter(
            "proj-1",
            "sms",
            "vonage",
            &serde_json::json!({"api_key": "k", "api_secret": "s", "from_number": "Notifi"}),
        );
        assert!(vonage.is_some());
        assert_eq!(vonage.unwrap().provider_name(), "vonage");
    }

    #[test]
    fn factory_arms_the_other_channel_wrappers() {
        let f = DefaultAdapterFactory::default();

        // The other channel's onesignal envelope arms both phone
        // families from one saved config — the arm rides the device
        // family's controller, tagged behind the "other" channel name.
        let onesignal = f.adapter(
            "proj-1",
            "other",
            "onesignal",
            &serde_json::json!({"provider": "onesignal", "app_id": "appid", "api_key": "k"}),
        );
        assert!(onesignal.is_some());
        let onesignal = onesignal.unwrap();
        assert_eq!(onesignal.channel(), "other");
        assert_eq!(onesignal.provider_name(), "onesignal");

        // Pushy's wrapper does the same — one api key serves phone
        // families plus web, tagged behind the "other" channel name.
        let pushy = f.adapter(
            "proj-1",
            "other",
            "pushy",
            &serde_json::json!({"provider": "pushy", "api_key": "k"}),
        );
        assert!(pushy.is_some());
        let pushy = pushy.unwrap();
        assert_eq!(pushy.channel(), "other");
        assert_eq!(pushy.provider_name(), "pushy");
        assert!(
            f.adapter(
                "proj-1",
                "other",
                "pushy",
                &serde_json::json!({"provider": "pushy"})
            )
            .is_none()
        );

        // Blanks stay None, and an envelope whose tag disagrees with
        // the arm resolves to None (never a wrong-vendor adapter).
        assert!(
            f.adapter(
                "proj-1",
                "other",
                "onesignal",
                &serde_json::json!({"provider": "onesignal", "app_id": "appid"})
            )
            .is_none()
        );
        assert!(
            f.adapter(
                "proj-1",
                "other",
                "onesignal",
                &serde_json::json!({"provider": "relay", "target_url": "https://x/hook"})
            )
            .is_none()
        );

        // The relay recipe arms from its url — the method / headers /
        // template ride along to the webhook family's provider.
        let relay = f.adapter(
            "proj-1",
            "other",
            "relay",
            &serde_json::json!({
                "provider": "relay",
                "target_url": "https://x/hook",
                "method": "POST",
                "payload_template": "{\"t\":\"{{title}}\"}"
            }),
        );
        assert!(relay.is_some());
        let relay = relay.unwrap();
        assert_eq!(relay.channel(), "other");
        assert_eq!(relay.provider_name(), "relay");
        assert!(
            f.adapter(
                "proj-1",
                "other",
                "relay",
                &serde_json::json!({
                    "provider": "relay",
                    "target_url": "",
                    "method": "POST",
                    "payload_template": "{}"
                })
            )
            .is_none()
        );
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

    #[test]
    fn gateway_factory_arms_store_only() {
        // An un-wired factory has nothing for gateway; a wired one arms the
        // full recipe. The unit-defaulted factory holds no store, so the
        // full-recipe case is the only one that lands here.
        assert!(
            factory_with_store()
                .adapter(
                    "proj-1",
                    "other",
                    "gateway",
                    &serde_json::json!({
                        "provider": "gateway",
                        "topic": "gw-xk4p2m9tq7v3", "secret_key": "gen-32chars-long-secret"
                    })
                )
                .is_some()
        );

        // A recipe missing either route half refuses even with the store.
        assert!(
            factory_with_store()
                .adapter(
                    "proj-1",
                    "other",
                    "gateway",
                    &serde_json::json!({"provider": "gateway", "topic": "t"})
                )
                .is_none()
        );
        assert!(
            factory_with_store()
                .adapter(
                    "proj-1",
                    "other",
                    "gateway",
                    &serde_json::json!({"provider": "gateway", "secret_key": "only-key"})
                )
                .is_none()
        );
    }

    #[tokio::test]
    async fn gateway_send_queues_onto_the_route() {
        let factory = factory_with_store();
        let adapter = factory
            .adapter(
                "proj-1",
                "other",
                "gateway",
                &serde_json::json!({
                    "provider": "gateway",
                    "topic": "gw-xk4p2m9tq7v3", "secret_key": "gen-32chars-long-secret",
                    "mode": "polling"
                }),
            )
            .unwrap();
        assert_eq!(adapter.channel(), "other");
        assert_eq!(adapter.provider_name(), "gateway");

        let receipt = adapter.send(&message()).await.expect("gateway send queues");
        assert_eq!(receipt.provider, "gateway");
        assert!(receipt.provider_message_id.is_none());

        let mut store = GATEWAY_LOG.lock().unwrap();
        assert_eq!(store.len(), 1, "exactly one row queued");
        let single = store.pop().unwrap();
        assert_eq!(single.0, "gw-xk4p2m9tq7v3");
        assert_eq!(single.1, "gen-32chars-long-secret");
        assert_eq!(single.2, "Hi");
        assert_eq!(single.3, "Body");
        assert!(store.is_empty(), "exactly one row queued");
    }
}
