//! Window destination — Windows devices via WNS.

pub mod wns;

use futures::future::BoxFuture;

/// Window's inner sender contract: the flavor decides how the WNS
/// channel URI is reached.
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
            Self::Wns { .. } => "wns",
        }
    }

    /// Builds the flavor's concrete sender.
    pub(crate) fn into_sender(self) -> Box<dyn WindowSender> {
        match self {
            Provider::Wns {
                client_id,
                client_secret,
                package_sid,
            } => Box::new(wns::WnsProvider::new(client_id, client_secret, package_sid)),
        }
    }
}

/// The destination's registry listing — kept in sync with `Provider`
/// by the family's golden test (add a flavor in both places).
pub fn vendors() -> Vec<super::VendorEntry> {
    vec![super::VendorEntry {
        id: "wns",
        name: "Windows Notification Service (WNS)",
        platforms: &["windows"],
        icon_url: Some("https://cdn.brandfetch.io/microsoft.com/w/512/h/512/theme/dark/icon.jpeg"),
        docs_url: Some(
            "https://learn.microsoft.com/en-us/windows/apps/develop/notifications/push-notifications/wns-overview",
        ),
        fields: vec![
            super::VendorField {
                key: "client_id",
                label: "Client ID",
                ty: super::FieldType::Text,
                required: true,
            },
            super::VendorField {
                key: "client_secret",
                label: "Client Secret",
                ty: super::FieldType::Password,
                required: true,
            },
            super::VendorField {
                key: "package_sid",
                label: "Package SID",
                ty: super::FieldType::Text,
                required: true,
            },
        ],
    }]
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
    fn provider_names_cover_the_family() {
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
