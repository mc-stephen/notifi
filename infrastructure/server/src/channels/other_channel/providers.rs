//! The other family's registry surface — the dashboard's provider
//! registry derives the "Other" section from this catalog, the same
//! compiler-backed derivation the device family uses. No vendor logic
//! lives here: each entry describes the wrapper this channel arms, and
//! the card list below is the dashboard-facing half of
//! `channels/other_channel`.

use crate::channels::device_channel::providers::{FieldType, VendorEntry, VendorField};

/// The other channel's dashboard catalog, in card order.
pub fn catalog() -> Vec<VendorEntry> {
    vec![
        VendorEntry {
            id: "onesignal",
            name: "OneSignal (Android + Apple)",
            platforms: &["android", "apple"],
            icon_url: Some(
                "https://cdn.brandfetch.io/onesignal.com/w/512/h/512/theme/dark/icon.jpeg",
            ),
            docs_url: Some("https://documentation.onesignal.com/"),
            fields: vec![
                VendorField {
                    key: "app_id",
                    label: "App ID",
                    ty: FieldType::Text,
                    required: true,
                },
                VendorField {
                    key: "api_key",
                    label: "API Key",
                    ty: FieldType::Password,
                    required: false,
                },
                VendorField {
                    key: "rest_api_key",
                    label: "REST API Key",
                    ty: FieldType::Password,
                    required: false,
                },
            ],
        },
        VendorEntry {
            id: "relay",
            name: "Webhook (Relay)",
            platforms: &[],
            icon_url: None,
            docs_url: Some("https://docs.notifi.dev/channels/webhook-relay"),
            fields: vec![
                VendorField {
                    key: "target_url",
                    label: "Target URL",
                    ty: FieldType::Text,
                    required: true,
                },
                VendorField {
                    key: "method",
                    label: "HTTP Method (e.g. POST)",
                    ty: FieldType::Text,
                    required: true,
                },
                VendorField {
                    key: "headers",
                    label: "Headers (JSON object, optional)",
                    ty: FieldType::Text,
                    required: false,
                },
                VendorField {
                    key: "payload_template",
                    label: "Payload Template (title, body, project, timestamp)",
                    ty: FieldType::Text,
                    required: true,
                },
            ],
        },
    ]
}

#[cfg(test)]
mod catalog_tests {
    use super::*;

    #[test]
    fn catalog_matches_the_wrapped_flavors() {
        let entries = catalog();
        let ids: Vec<&str> = entries.iter().map(|v| v.id).collect();
        assert_eq!(ids, vec!["onesignal", "relay"]);

        // One card serves both phone families — the whole point of the
        // other channel's onesignal wrapper.
        let onesignal = &entries[0];
        assert_eq!(onesignal.platforms, &["android", "apple"]);

        // The relay recipe carries every render knob the webhook
        // family's provider takes; headers stay optional.
        let relay_fields: Vec<&str> = entries[1].fields.iter().map(|f| f.key).collect();
        assert_eq!(
            relay_fields,
            vec!["target_url", "method", "headers", "payload_template"]
        );
        let required: Vec<(&str, bool)> = entries[1]
            .fields
            .iter()
            .map(|f| (f.key, f.required))
            .collect();
        assert_eq!(
            required,
            vec![
                ("target_url", true),
                ("method", true),
                ("headers", false),
                ("payload_template", true),
            ]
        );
    }
}
