use serde::Deserialize;
use std::collections::HashMap;

/// Saved provider config for the other channel — provider name first
/// (the `provider` tag), vendor fields after, the same envelope shape
/// the chat channel owns. No vendor logic is duplicated: the onesignal
/// variant wraps the device family's `OneSignalConfig`, the relay
/// variant feeds the webhook family's `RelayProvider`.
///
/// Per-send the recipient is the vendor's address: a OneSignal
/// subscription id serves android AND apple tokens alike. The relay
/// recipe carries its target URL in this saved config, so the per-send
/// address is informational only. The function flavor stays internal —
/// no dashboard card offers it yet.
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
