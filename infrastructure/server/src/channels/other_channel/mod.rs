//! The "other" channel — vendors that belong to no single platform
//! family. OneSignal speaks both phone families at once (its
//! subscription ids are one platform-agnostic namespace), so its one
//! saved config arms android and apple destinations together; pushy in
//! turn serves the phone families plus web from one api key. The
//! webhook relay posts raw payloads to services Notifi does not
//! natively integrate; the gateway is the hosted pull-route (your app
//! polls a Notifi-hosted queue). This folder owns every card's file —
//! the push gateways' controllers live here (onesignal.rs, pushy.rs),
//! the retired webhook family's relay.rs moved in from its old folder
//! (the family's placeholder function flavor was removed with it —
//! cardless means gone), and the gateway lives here too: gateway.rs
//! holds its send and the poll read behind the secret route, while the
//! queue contract stays `ports::gateway` and the route door is
//! `src/api/routes/gateway.rs`. The webhook relay's integration guide
//! lives behind `docs/` (moved with its controller). A provider that
//! outgrows the wrapper earns a real channel of its own.

pub mod gateway;
pub mod onesignal;
pub mod pushy;
pub mod relay;

use serde::Deserialize;
use std::collections::HashMap;

/// Saved provider config for the other channel — provider name first
/// (the `provider` tag), vendor fields after, the same envelope shape
/// the chat channel owns. No vendor logic is duplicated: the onesignal
/// and relay variants' controllers live in this folder
/// (onesignal.rs, relay.rs).
///
/// Per-send the recipient is the vendor's address: a OneSignal
/// subscription id serves android AND apple tokens alike. The relay
/// recipe carries its target URL in this saved config, so the per-send
/// address is informational only.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "provider")]
pub enum OtherChannelConfig {
    // Either key alone must arm — the dashboard may save either field
    // first, the same tolerance the device arms hand-extract for.
    #[serde(rename = "onesignal")]
    OneSignal {
        app_id: String,
        #[serde(default)]
        api_key: String,
        #[serde(default)]
        rest_api_key: Option<String>,
    },
    #[serde(rename = "pushy")]
    Pushy {
        api_key: String,
        #[serde(default)]
        app_id: Option<String>,
    },
    #[serde(rename = "relay")]
    Relay {
        target_url: String,
        method: String,
        #[serde(default, deserialize_with = "headers_from_json")]
        headers: HashMap<String, String>,
        payload_template: String,
    },
    #[serde(rename = "gateway")]
    Gateway {
        /// The secret-route topic (the poll URL's path piece) — filled
        /// by the dashboard at connect, never typed by the user.
        #[serde(default)]
        topic: String,
        #[serde(default)]
        secret_key: String,
        #[serde(default)]
        mode: Option<String>,
    },
}

impl OtherChannelConfig {
    /// A config whose credentials are all blank parses fine but
    /// connects to nothing — the adapter factory treats that the same
    /// as an absent vendor (matches the chat channel's guard).
    pub fn carries_credentials(&self) -> bool {
        match self {
            Self::OneSignal {
                app_id,
                api_key,
                rest_api_key,
            } => {
                if app_id.trim().is_empty() {
                    return false;
                }
                rest_api_key.as_deref().is_some_and(|k| !k.is_empty()) || !api_key.trim().is_empty()
            }
            Self::Pushy { api_key, .. } => !api_key.trim().is_empty(),
            Self::Relay { target_url, .. } => !target_url.trim().is_empty(),
            Self::Gateway {
                topic, secret_key, ..
            } => !topic.trim().is_empty() && !secret_key.trim().is_empty(),
        }
    }
}

/// Headers travel loose: the connect form collects them as one text
/// field, so the saved value can be a JSON object, a string holding a
/// JSON object, or empty — all mean the header map.
fn headers_from_json<'de, D>(d: D) -> Result<HashMap<String, String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(d)?;
    match value {
        serde_json::Value::Null => Ok(HashMap::new()),
        serde_json::Value::Object(map) => Ok(into_string_map(map)),
        serde_json::Value::String(s) if s.trim().is_empty() => Ok(HashMap::new()),
        serde_json::Value::String(s) => {
            let map: serde_json::Map<String, serde_json::Value> =
                serde_json::from_str(&s).map_err(serde::de::Error::custom)?;
            Ok(into_string_map(map))
        }
        _ => Err(serde::de::Error::custom(
            "headers must be a JSON object or a JSON object string",
        )),
    }
}

fn into_string_map(map: serde_json::Map<String, serde_json::Value>) -> HashMap<String, String> {
    map.into_iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string())))
        .collect()
}

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

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn envelope_parses_either_wrapped_flavor() {
        let one: OtherChannelConfig =
            serde_json::from_str(r#"{"provider":"onesignal","app_id":"a","rest_api_key":"k"}"#)
                .expect("onesignal envelope parses with defaults");
        assert!(one.carries_credentials());

        let relay: OtherChannelConfig = serde_json::from_str(
            r#"{"provider":"relay","target_url":"https://x/hook","method":"POST","payload_template":"{}"}"#,
        )
        .expect("relay envelope parses with headers defaulted");
        assert!(relay.carries_credentials());
    }

    #[test]
    fn blank_credentials_do_not_count() {
        let blank: OtherChannelConfig =
            serde_json::from_str(r#"{"provider":"onesignal","app_id":"a"}"#)
                .expect("onesignal blank keys");
        assert!(!blank.carries_credentials());

        let blank: OtherChannelConfig = serde_json::from_str(
            r#"{"provider":"relay","target_url":"  ","method":"POST","payload_template":"{}"}"#,
        )
        .expect("relay blank url parses");
        assert!(!blank.carries_credentials());
    }

    #[test]
    fn relay_headers_travel_loose() {
        // The connect form posts headers as one text field — a string
        // holding the JSON — and an object or empty value is equally
        // fine; all mean the header map.
        let from_string: OtherChannelConfig = serde_json::from_str(
            r#"{"provider":"relay","target_url":"https://x/hook","method":"POST","headers":"{\"Authorization\":\"Bearer t\"}","payload_template":"{}"}"#,
        )
        .expect("string headers parse");
        let OtherChannelConfig::Relay { headers, .. } = from_string else {
            panic!("relay variant expected");
        };
        assert_eq!(
            headers.get("Authorization").map(String::as_str),
            Some("Bearer t")
        );

        let from_object: OtherChannelConfig = serde_json::from_str(
            r#"{"provider":"relay","target_url":"https://x/hook","method":"POST","headers":{"Content-Type":"application/json"},"payload_template":"{}"}"#,
        )
        .expect("object headers parse");
        let OtherChannelConfig::Relay { headers, .. } = from_object else {
            panic!("relay variant expected");
        };
        assert_eq!(
            headers.get("Content-Type").map(String::as_str),
            Some("application/json")
        );

        let empty: OtherChannelConfig = serde_json::from_str(
            r#"{"provider":"relay","target_url":"https://x/hook","method":"POST","headers":"","payload_template":"{}"}"#,
        )
        .expect("empty headers parse");
        let OtherChannelConfig::Relay { headers, .. } = empty else {
            panic!("relay variant expected");
        };
        assert!(headers.is_empty());
    }
}
