//! Device channel delivery — one channel family for everything that
//! lands on a user's device: native app push per destination (android,
//! apple, window) and browser push (web).
//!
//! The OS is the provider, not a channel of its own: one APNs recipe
//! covers an app's Apple installs, Windows walks WNS, and the
//! destination shapes genuinely differ (registration token / WNS
//! channel URI / web-push subscription), so the family dispatches by
//! variant rather than forcing one uniform message type. Each
//! destination module (declared inline right below, with its vendor
//! files grouped in the destination's folder — `android/fcm.rs`,
//! `apple/apns.rs`, …) carries its `Provider` enum plus a `vendors()`
//! registry listing beside it. Onboarding docs for vendors live in
//! `docs/`.
//!
//! The dashboard's provider registry (`/app/providers`) derives its
//! device rows from [`catalog()`] — adding a vendor is its provider
//! file plus the destination's `Provider` enum and `vendors()` listing
//! (golden test `catalog_pins_every_family_variant` pins both lists to
//! each other). Cross-platform vendors (OneSignal, Pushy) belong to
//! no single destination — their cards and their controllers live in
//! `other_channel`, and legacy android/apple saves keep delivering
//! through their device arms, pointed at those controllers. Device
//! tokens are recipient data — the recipient's
//! contacts blob carries a `devices` array (§25); delivery is
//! adapter-wired in `infra::delivery_adapters` (every registry card
//! sends), and the blob walker that fans that array out per install is
//! still the send layer's pending job.

pub mod android {
    //! Android destination — its native vendor is FCM. The
    //! cross-platform gateways (Pushy, OneSignal) are the other
    //! channel's cards, with their controllers in that folder; gateway
    //! saves under android keep their delivery arms, pointed there.

    pub mod fcm;

    /// The Android destination's own message shape: an FCM-style
    /// registration token plus the notification copy.
    pub struct AndroidMessage {
        pub token: String,
        pub title: String,
        pub body: String,
    }

    /// All supported Android push providers. THIS IS THE SOURCE OF TRUTH.
    #[derive(Debug, Clone)]
    pub enum Provider {
        Fcm(fcm::FcmConfig),
    }

    impl Provider {
        pub fn provider_name(&self) -> &'static str {
            match self {
                Self::Fcm(_) => "fcm",
            }
        }
    }

    /// The destination's registry listing — kept in sync with `Provider`
    /// by the family's golden test (add a vendor in both places).
    pub fn vendors() -> Vec<super::VendorEntry> {
        vec![super::VendorEntry {
            id: "fcm",
            name: "Firebase Cloud Messaging",
            platforms: &["android"],
            icon_url: Some("https://api.iconify.design/logos/firebase-icon.svg"),
            docs_url: Some("https://firebase.google.com/docs/cloud-messaging"),
            fields: vec![super::VendorField {
                key: "service_account_path",
                label: "Service account key path (JSON)",
                ty: super::FieldType::Text,
                required: true,
            }],
        }]
    }

    #[cfg(test)]
    mod android_dest_tests {
        use super::*;

        #[test]
        fn provider_names_cover_all_variants() {
            let fcm = Provider::Fcm(fcm::FcmConfig {
                service_account_path: std::path::PathBuf::from("/x/sa.json"),
            });
            assert_eq!(fcm.provider_name(), "fcm");
        }
    }
}

pub mod apple {
    //! Apple destination — everything that can push to an Apple device.
    //!
    //! One APNs recipe covers the app's Apple installs: the team-scoped
    //! `.p8` key signs every push, the recipe's bundle id names the app,
    //! and each install (iPhone, iPad, Mac for a universal app) registers
    //! its own token that Apple routes. Mac installs ride the same recipe
    //! unless an app ships a separate Mac bundle id — that day, a second
    //! recipe earns its place back.

    pub mod apns;

    /// All supported Apple push providers. THIS IS THE SOURCE OF TRUTH.
    #[derive(Debug, Clone)]
    pub enum Provider {
        Apns(apns::IosConfig),
    }

    impl Provider {
        pub fn provider_name(&self) -> &'static str {
            match self {
                Self::Apns(_) => "apns",
            }
        }
    }

    /// The destination's registry listing — kept in sync with `Provider`
    /// by the family's golden test (add a vendor in both places).
    pub fn vendors() -> Vec<super::VendorEntry> {
        vec![super::VendorEntry {
            id: "apns",
            name: "Apple Push (iPhone, iPad & Mac)",
            platforms: &["ios", "ipados", "macos"],
            icon_url: Some("https://cdn.brandfetch.io/apple.com/w/512/h/512/theme/dark/icon.jpeg"),
            docs_url: Some("https://developer.apple.com/documentation/usernotifications"),
            fields: apns_fields(),
        }]
    }

    /// The APNs recipe's connect-form fields — they match
    /// [`apns::IosConfig`].
    fn apns_fields() -> Vec<super::VendorField> {
        vec![
            super::VendorField {
                key: "key_id",
                label: "Key ID",
                ty: super::FieldType::Text,
                required: true,
            },
            super::VendorField {
                key: "team_id",
                label: "Team ID",
                ty: super::FieldType::Text,
                required: true,
            },
            super::VendorField {
                key: "bundle_id",
                label: "Bundle ID",
                ty: super::FieldType::Text,
                required: true,
            },
            super::VendorField {
                key: "p8_key_path",
                label: "APNs auth key path (.p8)",
                ty: super::FieldType::Text,
                required: true,
            },
            super::VendorField {
                key: "sandbox",
                label: "Sandbox mode",
                ty: super::FieldType::Boolean,
                required: false,
            },
        ]
    }

    #[cfg(test)]
    mod apple_dest_tests {
        use super::*;

        #[test]
        fn provider_names_cover_all_variants() {
            let apns = Provider::Apns(apns::IosConfig {
                key_id: "k".to_string(),
                team_id: "t".to_string(),
                bundle_id: "b".to_string(),
                p8_key_path: std::path::PathBuf::from("/k.p8"),
                sandbox: false,
            });
            assert_eq!(apns.provider_name(), "apns");

            // One recipe covers the app's Apple installs — the universal
            // app story: the carried platforms say so on the card.
            let apns_row = &vendors()[0];
            assert_eq!(apns_row.id, "apns");
            assert_eq!(apns_row.platforms, &["ios", "ipados", "macos"]);
            assert_eq!(apns_row.name, "Apple Push (iPhone, iPad & Mac)");
        }
    }
}

pub mod web {
    //! Web destination — browser push via the VAPID-signed web-push flow.

    pub mod web_push;

    /// All supported web (browser push) providers. THIS IS THE SOURCE OF TRUTH.
    #[derive(Debug, Clone)]
    pub enum Provider {
        WebPush(web_push::WebConfig),
    }

    impl Provider {
        pub fn provider_name(&self) -> &'static str {
            match self {
                Self::WebPush(_) => "web_push",
            }
        }
    }

    /// The destination's registry listing — kept in sync with `Provider`
    /// by the family's golden test.
    pub fn vendors() -> Vec<super::VendorEntry> {
        vec![super::VendorEntry {
            id: "web_push",
            name: "Web Push (VAPID)",
            platforms: &["web"],
            icon_url: None,
            docs_url: Some("https://datatracker.ietf.org/doc/html/rfc8292"),
            fields: vec![
                super::VendorField {
                    key: "vapid_public_key",
                    label: "VAPID public key",
                    ty: super::FieldType::Text,
                    required: true,
                },
                super::VendorField {
                    key: "vapid_private_key",
                    label: "VAPID private key",
                    ty: super::FieldType::Password,
                    required: true,
                },
                super::VendorField {
                    key: "contact_email",
                    label: "Contact email",
                    ty: super::FieldType::Email,
                    required: true,
                },
            ],
        }]
    }

    #[cfg(test)]
    mod web_dest_tests {
        use super::*;

        #[test]
        fn provider_name_is_web_push() {
            let p = Provider::WebPush(web_push::WebConfig {
                vapid_public_key: "p".to_string(),
                vapid_private_key: "k".to_string(),
                contact_email: "a@b.c".to_string(),
            });
            assert_eq!(p.provider_name(), "web_push");
        }
    }
}

pub mod window {
    //! Window destination — Windows devices via WNS.

    pub mod wns {
        mod auth;
        mod xml;

        use super::WindowSender;
        pub use auth::WnsAuthenticator;
        use futures::future::BoxFuture;
        use reqwest::Client;
        use serde::Deserialize;
        use std::sync::Arc;
        use xml::build_toast_xml;

        /// WNS configuration — Azure AD app credentials the token
        /// exchange uses.
        #[derive(Debug, Clone, Deserialize)]
        pub struct WnsConfig {
            pub client_id: String,
            pub client_secret: String,
            pub package_sid: String,
        }

        pub struct WnsProvider {
            auth: Arc<WnsAuthenticator>,
            client: Client,
        }

        impl WnsProvider {
            pub fn new(client_id: String, client_secret: String, package_sid: String) -> Self {
                Self {
                    auth: Arc::new(WnsAuthenticator::new(client_id, client_secret, package_sid)),
                    client: Client::new(),
                }
            }
        }

        impl WindowSender for WnsProvider {
            fn send(
                &self,
                target: &str,
                title: &str,
                body: &str,
            ) -> BoxFuture<'static, Result<(), String>> {
                let auth = self.auth.clone();
                let client = self.client.clone();
                let uri = target.to_string();
                let xml = build_toast_xml(title, body);

                Box::pin(async move {
                    let token = auth.get_token().await?;

                    let response = client
                        .post(uri)
                        .header("Authorization", format!("Bearer {}", token))
                        .header("X-WNS-Type", "wns/toast")
                        .header("Content-Type", "text/xml")
                        .body(xml)
                        .send()
                        .await
                        .map_err(|e| format!("WNS request failed: {e}"))?;

                    if response.status().is_success() {
                        Ok(())
                    } else {
                        let err = response.text().await.unwrap_or_default();
                        Err(format!("WNS error: {err}"))
                    }
                })
            }
        }
    }

    use futures::future::BoxFuture;

    /// Window's inner sender contract: the flavor decides how the WNS
    /// channel URI is reached.
    pub trait WindowSender: Send + Sync {
        fn send(
            &self,
            target: &str,
            title: &str,
            body: &str,
        ) -> BoxFuture<'static, Result<(), String>>;
    }

    /// The destination the window flavor sends to (URL or channel URI).
    pub struct WindowMessage {
        pub target: String,
        pub title: String,
        pub body: String,
    }

    /// Window device provider controller — the flavor picks the inner
    /// sender.
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
            icon_url: Some(
                "https://cdn.brandfetch.io/microsoft.com/w/512/h/512/theme/dark/icon.jpeg",
            ),
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
}

/// All supported device providers. THIS IS THE SOURCE OF TRUTH.
///
/// Each variant is a destination; the inner enum (same unit's `Provider`)
/// names the vendor serving it.
#[derive(Debug, Clone)]
pub enum DeviceProvider {
    Android(android::Provider),
    Apple(apple::Provider),
    Web(web::Provider),
    Window(window::Provider),
}

impl DeviceProvider {
    /// The destination half: which OS flavor receives the message.
    pub fn destination_name(&self) -> &'static str {
        match self {
            Self::Android(_) => "android",
            Self::Apple(_) => "apple",
            Self::Web(_) => "web",
            Self::Window(_) => "window",
        }
    }

    /// The vendor half: which delivery service carries it inside that
    /// destination.
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::Android(p) => p.provider_name(),
            Self::Apple(p) => p.provider_name(),
            Self::Web(p) => p.provider_name(),
            Self::Window(p) => p.provider_name(),
        }
    }
}

/// Which input a vendor's config field takes on the connect form. The
/// provider registry maps these one-to-one onto its own field type —
/// compiler-exhaustive on both sides.
#[derive(Debug, Clone, Copy)]
pub enum FieldType {
    Text,
    Password,
    Email,
    /// A dropdown with exactly these choices (id strings the config
    /// stores; the dashboard renders them as the picker's options).
    Select(&'static [&'static str]),
    Boolean,
    /// Saved into the recipe but never rendered — a value the dashboard
    /// generates itself at connect time (e.g. the gateway's route topic).
    Hidden,
}

/// One config field on a vendor's connect form.
#[derive(Debug, Clone)]
pub struct VendorField {
    pub key: &'static str,
    pub label: &'static str,
    pub ty: FieldType,
    pub required: bool,
}

/// One vendor's dashboard description. `id` mirrors the variant's
/// `provider_name()` — the golden test pins them together.
#[derive(Debug, Clone)]
pub struct VendorEntry {
    pub id: &'static str,
    pub name: &'static str,
    /// Device flavors the vendor serves (e.g. `["ios"]`).
    pub platforms: &'static [&'static str],
    /// Brand icon (same icon/brand CDNs the base channels use). None
    /// for self-hosted or standard-not-a-company entries — the
    /// dashboard falls back to a name glyph.
    pub icon_url: Option<&'static str>,
    /// Where the dashboard's "Docs" link points: the vendor's own
    /// integration page, or the platform/protocol spec for no-company
    /// rows. Verified-alive URLs only.
    pub docs_url: Option<&'static str>,
    pub fields: Vec<VendorField>,
}

/// One destination's dashboard-facing row: channel id/name plus the
/// destination's vendors. Each destination's `vendors()` lives beside
/// its `Provider` enum, so a new vendor is a provider-file edit plus
/// that one enum block — the golden test keeps both lists pinned.
#[derive(Debug, Clone)]
pub struct DestCatalog {
    pub destination: &'static str,
    pub title: &'static str,
    pub vendors: Vec<VendorEntry>,
}

/// The device family's dashboard catalog — one entry per destination.
/// The provider registry (`api/routes/app/providers`) derives its
/// device rows from this; nothing about vendors is typed out there.
pub fn catalog() -> Vec<DestCatalog> {
    vec![
        DestCatalog {
            destination: "android",
            title: "Android",
            vendors: android::vendors(),
        },
        DestCatalog {
            destination: "apple",
            title: "Apple",
            vendors: apple::vendors(),
        },
        DestCatalog {
            destination: "web",
            title: "Web Push",
            vendors: web::vendors(),
        },
        DestCatalog {
            destination: "window",
            title: "Windows",
            vendors: window::vendors(),
        },
    ]
}

#[cfg(test)]
mod device_dest_tests {
    use super::*;

    #[test]
    fn destination_names_cover_all_family_members() {
        let names: Vec<&str> = [
            DeviceProvider::Android(android::Provider::Fcm(fcm_like_config())).destination_name(),
            DeviceProvider::Apple(apple::Provider::Apns(apns_like_config())).destination_name(),
            DeviceProvider::Web(web::Provider::WebPush(web_push_like_config())).destination_name(),
            DeviceProvider::Window(window::Provider::Wns {
                client_id: "c".to_string(),
                client_secret: "s".to_string(),
                package_sid: "p".to_string(),
            })
            .destination_name(),
        ]
        .to_vec();
        assert_eq!(names, ["android", "apple", "web", "window"]);
    }

    fn fcm_like_config() -> android::fcm::FcmConfig {
        android::fcm::FcmConfig {
            service_account_path: std::path::PathBuf::from("/x/sa.json"),
        }
    }

    fn apns_like_config() -> apple::apns::IosConfig {
        apple::apns::IosConfig {
            key_id: "k".to_string(),
            team_id: "t".to_string(),
            bundle_id: "b".to_string(),
            p8_key_path: std::path::PathBuf::from("/k.p8"),
            sandbox: false,
        }
    }

    fn web_push_like_config() -> web::web_push::WebConfig {
        web::web_push::WebConfig {
            vapid_public_key: "p".to_string(),
            vapid_private_key: "k".to_string(),
            contact_email: "a@b.c".to_string(),
        }
    }

    /// The pin: every Provider variant (destination × vendor) must be
    /// listed in `catalog()`, and every listed vendor must exist as a
    /// variant on its destination's enum. A vendor added to an enum
    /// without its `vendors()` entry — or the reverse — fails here.
    #[test]
    fn catalog_pins_every_family_variant() {
        let variants: Vec<(&str, &str)> = vec![
            (
                "android",
                android::Provider::Fcm(fcm_like_config()).provider_name(),
            ),
            (
                "apple",
                apple::Provider::Apns(apns_like_config()).provider_name(),
            ),
            (
                "web",
                web::Provider::WebPush(web_push_like_config()).provider_name(),
            ),
            (
                "window",
                window::Provider::Wns {
                    client_id: "c".to_string(),
                    client_secret: "s".to_string(),
                    package_sid: "p".to_string(),
                }
                .provider_name(),
            ),
        ];
        let catalog = catalog();
        for (destination, id) in &variants {
            assert!(
                catalog
                    .iter()
                    .find(|c| c.destination == *destination)
                    .is_some_and(|c| c.vendors.iter().any(|v| v.id == *id)),
                "{destination}/{id} missing from the device catalog — add it beside the Provider enum"
            );
        }
        // Reverse: nothing listed without an enum variant behind it.
        for dest in &catalog {
            for vendor in &dest.vendors {
                assert!(
                    variants
                        .iter()
                        .any(|(d, id)| *d == dest.destination && id == &vendor.id),
                    "{}/{}: listed in the catalog but has no Provider variant",
                    dest.destination,
                    vendor.id
                );
            }
        }
    }
}
