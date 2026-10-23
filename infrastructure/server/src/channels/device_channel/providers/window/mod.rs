//! Window destination — Windows devices, webhook or WNS flavor.

pub mod webhook;
pub mod wns;

use futures::future::BoxFuture;

/// Window's inner sender contract: the flavor (webhook or WNS) decides
/// how `target` (webhook URL or WNS channel URI) is reached.
pub trait WindowSender: Send + Sync {
    fn send(&self, target: &str, title: &str, body: &str)
    -> BoxFuture<'static, Result<(), String>>;
}

/// The destination the window flavor sends to (URL or channel URI).
pub struct WindowMessage {
    pub target: String,
    pub title: String,
    pub body: String,
}

/// Window device provider controller — the flavor picks the inner sender.
pub struct WindowProvider {
    sender: Box<dyn WindowSender>,
}

/// All supported Windows push providers. THIS IS THE SOURCE OF TRUTH.
#[derive(Debug, Clone)]
pub enum Provider {
    /// Webhook flavor needs no credentials — the target URL carries the
    /// machine's registration.
    Webhook,
    /// WNS signs token requests against Microsoft's identity endpoint
    /// via client credentials.
    Wns {
        client_id: String,
        client_secret: String,
        package_sid: String,
    },
}

impl Provider {
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::Webhook => "webhook",
            Self::Wns { .. } => "wns",
        }
    }

    /// Builds the flavor's concrete sender.
    pub(crate) fn into_sender(self) -> Box<dyn WindowSender> {
        match self {
            Provider::Webhook => Box::new(webhook::WebhookProvider::new()),
            Provider::Wns {
                client_id,
                client_secret,
                package_sid,
            } => Box::new(wns::WnsProvider::new(client_id, client_secret, package_sid)),
        }
    }
}

impl WindowProvider {
    pub fn new(provider: Provider) -> Self {
        Self {
            sender: provider.into_sender(),
        }
    }

    pub async fn send_notification(&self, msg: &WindowMessage) -> Result<(), String> {
        self.sender.send(&msg.target, &msg.title, &msg.body).await
    }
}

#[cfg(test)]
mod window_tests {
    use super::*;

    #[test]
    fn provider_names_cover_both_flavors() {
        assert_eq!(Provider::Webhook.provider_name(), "webhook");
        assert_eq!(
            Provider::Wns {
                client_id: "c".to_string(),
                client_secret: "s".to_string(),
                package_sid: "p".to_string(),
            }
            .provider_name(),
            "wns"
        );
    }

    #[tokio::test]
    async fn webhook_flavor_posts_title_body_payload() {
        let stub = crate::testing::http_stub::start(200, "OK", "").await;
        WindowProvider::new(Provider::Webhook)
            .send_notification(&WindowMessage {
                target: format!("http://127.0.0.1:{}/wa", stub.port),
                title: "Hi".to_string(),
                body: "Body".to_string(),
            })
            .await
            .expect("window webhook 200 delivers");
        let raw = stub.request().await.expect("request captured");
        assert!(raw.contains("POST /wa"), "target path: {raw}");
        assert!(raw.contains(r#""title":"Hi""#), "payload: {raw}");
        assert!(raw.contains(r#""body":"Body""#), "payload: {raw}");
    }

    #[tokio::test]
    async fn webhook_flavor_surfaces_error_body() {
        let stub = crate::testing::http_stub::start(404, "Not Found", "no such machine").await;
        let err = WindowProvider::new(Provider::Webhook)
            .send_notification(&WindowMessage {
                target: format!("http://127.0.0.1:{}/wa", stub.port),
                title: "t".to_string(),
                body: "b".to_string(),
            })
            .await
            .expect_err("404 is a failed window delivery");
        assert!(err.starts_with("Webhook error:"), "prefix: {err}");
        assert!(err.contains("no such machine"), "body surfaced: {err}");
    }

    #[tokio::test]
    #[ignore = "live WNS delivery — needs NOTIFI_WNS_CLIENT_ID, NOTIFI_WNS_CLIENT_SECRET, \
                NOTIFI_WNS_PACKAGE_SID, and a real WNS channel URI"]
    async fn live_wns_delivery() {
        let provider_token = Provider::Wns {
            client_id: std::env::var("NOTIFI_WNS_CLIENT_ID").unwrap_or_default(),
            client_secret: std::env::var("NOTIFI_WNS_CLIENT_SECRET").unwrap_or_default(),
            package_sid: std::env::var("NOTIFI_WNS_PACKAGE_SID").unwrap_or_default(),
        };
        let target =
            std::env::var("NOTIFI_WNS_CHANNEL_URI").expect("NOTIFI_WNS_CHANNEL_URI not set");
        WindowProvider::new(provider_token)
            .send_notification(&WindowMessage {
                target,
                title: "Notifi live-check".to_string(),
                body: "Direct WNS delivery check".to_string(),
            })
            .await
            .expect("real WNS accepted the push");
    }
}
