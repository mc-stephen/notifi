//! Apple destination — everything that can push to an Apple device.
//!
//! One APNs recipe covers the app's Apple installs: the team-scoped
//! `.p8` key signs every push, the recipe's bundle id names the app,
//! and each install (iPhone, iPad, Mac for a universal app) registers
//! its own token that Apple routes. Mac installs ride the same recipe
//! unless an app ships a separate Mac bundle id — that day, a second
//! recipe earns its place back.

pub mod apns;
pub mod onesignal;

/// All supported Apple push providers. THIS IS THE SOURCE OF TRUTH.
#[derive(Debug, Clone)]
pub enum Provider {
    Apns(apns::IosConfig),
    OneSignal(onesignal::OneSignalConfig),
}

impl Provider {
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::Apns(_) => "apns",
            Self::OneSignal(_) => "onesignal",
        }
    }
}

/// The destination's registry listing — kept in sync with `Provider`
/// by the family's golden test (add a vendor in both places).
pub fn vendors() -> Vec<super::VendorEntry> {
    vec![
        super::VendorEntry {
            id: "apns",
            name: "Apple Push (iPhone, iPad & Mac)",
            platforms: &["ios", "ipados", "macos"],
            icon_url: Some("https://cdn.brandfetch.io/apple.com/w/512/h/512/theme/dark/icon.jpeg"),
            docs_url: Some("https://developer.apple.com/documentation/usernotifications"),
            fields: apns_fields(),
        },
        super::VendorEntry {
            id: "onesignal",
            name: "OneSignal",
            platforms: &["ios", "macos"],
            icon_url: Some(
                "https://cdn.brandfetch.io/onesignal.com/w/512/h/512/theme/dark/icon.jpeg",
            ),
            docs_url: Some("https://documentation.onesignal.com/"),
            fields: one_signal_fields(),
        },
    ]
}

/// The APNs recipe's connect-form fields — they match
/// [`apple::apns::IosConfig`].
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

fn one_signal_fields() -> Vec<super::VendorField> {
    vec![
        super::VendorField {
            key: "app_id",
            label: "App ID",
            ty: super::FieldType::Text,
            required: true,
        },
        super::VendorField {
            key: "api_key",
            label: "API key",
            ty: super::FieldType::Password,
            required: true,
        },
        super::VendorField {
            key: "rest_api_key",
            label: "REST API key",
            ty: super::FieldType::Password,
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
