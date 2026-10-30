//! Device channel providers, grouped per destination.
//!
//! The OS is the destination; vendors live inside their destination's
//! folder (`android/fcm.rs`, `apple/apns.rs`, …). Destination shapes
//! genuinely differ (push token / endpoint / WNS channel URI /
//! web-push subscription), so the family dispatches by destination
//! instead of forcing one uniform message type. Each destination also
//! carries its `vendors()` registry listing beside the enum, and
//! [`catalog()`] pairs them for the dashboard's provider registry.

pub mod android;
pub mod apple;
pub mod web;
pub mod window;

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
            DeviceProvider::Window(window::Provider::Webhook).destination_name(),
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
                "android",
                android::Provider::Pushy(android::pushy::PushyConfig {
                    api_key: "k".to_string(),
                    app_id: None,
                })
                .provider_name(),
            ),
            (
                "android",
                android::Provider::OneSignal(android::onesignal::OneSignalConfig {
                    app_id: "a".to_string(),
                    api_key: "k".to_string(),
                    rest_api_key: None,
                })
                .provider_name(),
            ),
            (
                "apple",
                apple::Provider::Apns(apns_like_config()).provider_name(),
            ),
            (
                "apple",
                apple::Provider::OneSignal(apple::onesignal::OneSignalConfig {
                    app_id: "a".to_string(),
                    api_key: "k".to_string(),
                    rest_api_key: None,
                })
                .provider_name(),
            ),
            (
                "web",
                web::Provider::WebPush(web_push_like_config()).provider_name(),
            ),
            ("window", window::Provider::Webhook.provider_name()),
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
