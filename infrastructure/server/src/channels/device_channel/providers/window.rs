use super::window_webhook::WebhookProvider;
use super::wns::WnsProvider;
use super::{WindowMessage, WindowSender};
use serde::Deserialize;

/// Window (Windows device) configuration — webhook push or WNS.
///
/// The webhook flavor needs no credentials (the target URL carries the
/// machine's registration); WNS signs token requests against Microsoft's
/// identity endpoint via client credentials.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum WindowConfig {
    #[serde(rename = "webhook")]
    Webhook {},
    #[serde(rename = "wns")]
    Wns {
        client_id: String,
        client_secret: String,
        package_sid: String,
    },
}

impl WindowConfig {
    /// Builds the flavor's concrete sender.
    pub(crate) fn into_sender(self) -> Box<dyn WindowSender> {
        match self {
            WindowConfig::Webhook {} => Box::new(WebhookProvider::new()),
            WindowConfig::Wns {
                client_id,
                client_secret,
                package_sid,
            } => Box::new(WnsProvider::new(client_id, client_secret, package_sid)),
        }
    }
}

/// Window device provider controller — the flavor picks the inner sender.
pub struct WindowProvider {
    sender: Box<dyn WindowSender>,
}

impl WindowProvider {
    pub fn new(config: WindowConfig) -> Self {
        Self {
            sender: config.into_sender(),
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
    fn config_parses_both_tagged_flavors() {
        let webhook: WindowConfig =
            serde_json::from_str(r#"{"type":"webhook"}"#).expect("webhook parses");
        assert!(matches!(webhook, WindowConfig::Webhook {}));
        let wns: WindowConfig = serde_json::from_str(
            r#"{"type":"wns","client_id":"c","client_secret":"s","package_sid":"p"}"#,
        )
        .expect("wns parses");
        assert!(matches!(wns, WindowConfig::Wns { .. }));
    }

    #[tokio::test]
    async fn webhook_flavor_posts_title_body_payload() {
        let stub = crate::testing::http_stub::start(200, "OK", "").await;
        WindowProvider::new(WindowConfig::Webhook {})
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
        let err = WindowProvider::new(WindowConfig::Webhook {})
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
        let config = WindowConfig::Wns {
            client_id: std::env::var("NOTIFI_WNS_CLIENT_ID").expect("NOTIFI_WNS_CLIENT_ID not set"),
            client_secret: std::env::var("NOTIFI_WNS_CLIENT_SECRET")
                .expect("NOTIFI_WNS_CLIENT_SECRET not set"),
            package_sid: std::env::var("NOTIFI_WNS_PACKAGE_SID")
                .expect("NOTIFI_WNS_PACKAGE_SID not set"),
        };
        let target =
            std::env::var("NOTIFI_WNS_CHANNEL_URI").expect("NOTIFI_WNS_CHANNEL_URI not set");
        WindowProvider::new(config)
            .send_notification(&WindowMessage {
                target,
                title: "Notifi live-check".to_string(),
                body: "Direct WNS delivery check".to_string(),
            })
            .await
            .expect("real WNS accepted the push");
    }
}
