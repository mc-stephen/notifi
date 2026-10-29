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
    #[serde(rename = "relay")]
    Relay {
        target_url: String,
        method: String,
        #[serde(default)]
        headers: HashMap<String, String>,
        payload_template: String,
    },
    #[serde(rename = "function")]
    Function { function_id: String },
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
            Self::Relay { target_url, .. } => !target_url.trim().is_empty(),
            Self::Function { function_id } => !function_id.trim().is_empty(),
        }
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
}
