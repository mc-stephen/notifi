//! Apple destination — everything that can push to an iPhone or Mac.
//!
//! Both OSes ride the same APNs signing flow; the flavor lives in the
//! device configs (bundle ids differ), not in a separate controller.
//! Third-party vendors have config shells; their controllers fail
//! closed until wired.

pub mod apns;
pub mod onesignal;

/// All supported Apple push providers. THIS IS THE SOURCE OF TRUTH.
///
/// `Apns` and `ApnsMacos` are the same vendor against a different device
/// flavor (iPhone bundle vs Mac bundle); each carries its own APNs config.
#[derive(Debug, Clone)]
pub enum Provider {
    Apns(apns::IosConfig),
    ApnsMacos(apns::MacosConfig),
    OneSignal(onesignal::OneSignalConfig),
}

impl Provider {
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::Apns(_) => "apns",
            Self::ApnsMacos(_) => "apns_macos",
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
            name: "Apple Push Notification Service (iOS)",
            platforms: &["ios"],
            icon_url: Some("https://cdn.brandfetch.io/apple.com/w/512/h/512/theme/dark/icon.jpeg"),
            docs_url: Some("https://developer.apple.com/documentation/usernotifications"),
            fields: apns_fields(),
        },
        super::VendorEntry {
            id: "apns_macos",
            name: "Apple Push Notification Service (macOS)",
            platforms: &["macos"],
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

/// Same APNs signing flow per flavor — the bundle differs, the fields
/// match [`apple::apns::IosConfig`]/[`apple::apns::MacosConfig`].
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
        let macos = Provider::ApnsMacos(apns::MacosConfig {
            key_id: "k".to_string(),
            team_id: "t".to_string(),
            bundle_id: "b".to_string(),
            p8_key_path: std::path::PathBuf::from("/k.p8"),
            sandbox: false,
        });
        assert_eq!(macos.provider_name(), "apns_macos");
    }
}
