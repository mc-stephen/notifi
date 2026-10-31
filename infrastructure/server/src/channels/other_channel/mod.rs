//! The "other" channel — vendors that belong to no single platform
//! family. OneSignal speaks both phone families at once (its
//! subscription ids are one platform-agnostic namespace), so its one
//! saved config arms android and apple destinations together; pushy in
//! turn serves the phone families plus web from one api key. The
//! webhook relay posts raw payloads to services Notifi does not
//! natively integrate; the gateway is the hosted pull-route (your app
//! polls a Notifi-hosted queue). This folder owns every card's file —
//! the push gateways' controllers live here (onesignal.rs, pushy.rs),
//! the retired webhook family's relay.rs / function.rs moved in from
//! their old folder, and the gateway has its only home here
//! (`ports::gateway` queue + `src/api/routes/gateway.rs`). The webhook
//! relay and function flavors' integration guides live behind `docs/`
//! (they moved with the controllers). A provider that outgrows the
//! wrapper earns a real channel of its own.

pub mod config;
pub mod function;
pub mod onesignal;
pub mod pushy;
pub mod relay;

pub use config::OtherChannelConfig;

use crate::channels::device_channel::{FieldType, VendorEntry, VendorField};

/// The other family's registry surface — the dashboard's provider
/// registry derives the Other section from this catalog, the same
/// compiler-backed derivation the device family uses. No vendor logic
/// lives here: each entry describes the wrapper this channel arms, and
/// the card list below is the dashboard-facing half of
/// `channels/other_channel`.
///
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
            id: "pushy",
            name: "Pushy",
            platforms: &["android", "apple", "web"],
            icon_url: Some("https://cdn.brandfetch.io/pushy.me/w/512/h/512/theme/dark/icon.jpeg"),
            docs_url: Some("https://pushy.me/docs"),
            fields: vec![
                VendorField {
                    key: "api_key",
                    label: "API key",
                    ty: FieldType::Password,
                    required: true,
                },
                VendorField {
                    key: "app_id",
                    label: "App ID",
                    ty: FieldType::Text,
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
                    // A dropdown, not a free text: a typo a receiver
                    // cannot parse would silently fall back to POST.
                    label: "HTTP method",
                    ty: FieldType::Select(&["POST", "GET", "PUT", "PATCH", "DELETE"]),
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
        VendorEntry {
            id: "gateway",
            name: "Notifi Gateway",
            platforms: &[],
            icon_url: None,
            docs_url: Some("https://docs.notifi.dev/channels/gateway"),
            fields: vec![
                VendorField {
                    key: "topic",
                    label: "Route topic",
                    ty: FieldType::Hidden,
                    required: false,
                },
                VendorField {
                    key: "secret_key",
                    label: "Route secret",
                    ty: FieldType::Text,
                    required: true,
                },
                VendorField {
                    key: "mode",
                    label: "Delivery mode",
                    ty: FieldType::Select(&["polling", "https"]),
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
        assert_eq!(ids, vec!["onesignal", "pushy", "relay", "gateway"]);

        // One card serves both phone families — the whole point of the
        // other channel's onesignal wrapper; pushy joins it serving
        // phone plus web from one key.
        let onesignal = &entries[0];
        assert_eq!(onesignal.platforms, &["android", "apple"]);
        let pushy = &entries[1];
        assert_eq!(pushy.platforms, &["android", "apple", "web"]);

        // The relay recipe carries every render knob the webhook
        // family's provider takes; headers stay optional.
        let relay_fields: Vec<&str> = entries[2].fields.iter().map(|f| f.key).collect();
        assert_eq!(
            relay_fields,
            vec!["target_url", "method", "headers", "payload_template"]
        );
        let required: Vec<(&str, bool)> = entries[2]
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

        // The gateway: a cross-platform pull-route (platforms empty —
        // any machine that can poll subscribes). Its recipe is static —
        // the hidden topic generates at connect, the secret prefills,
        // the mode dropdown is the one real choice.
        let gateway = &entries[3];
        assert!(gateway.platforms.is_empty());
        let required: Vec<(&str, bool)> =
            gateway.fields.iter().map(|f| (f.key, f.required)).collect();
        assert_eq!(
            required,
            vec![("topic", false), ("secret_key", true), ("mode", true)]
        );
        let mode = gateway.fields.last().expect("mode field");
        assert!(matches!(mode.ty, FieldType::Select(&["polling", "https"])));

        // A free-typed method could silently misbehave — the relay's
        // method is a dropdown over the body-carrying set.
        let method = &entries[2]
            .fields
            .iter()
            .find(|f| f.key == "method")
            .expect("method field");
        assert!(matches!(
            method.ty,
            FieldType::Select(&["POST", "GET", "PUT", "PATCH", "DELETE"])
        ));
    }
}
