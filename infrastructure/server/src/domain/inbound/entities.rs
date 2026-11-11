//! Inbound webhook entities: node config extraction and token rules.
//!
//! Caller values win; the trigger node's stored config fills the gaps;
//! a picked template renders channel/title/body. Anything still missing
//! after fallback fails the fire closed.

/// Caller payload. Everything optional — the flow fills the gaps.
#[derive(Debug, Clone, Default)]
pub struct InboundPayload {
    pub channel: Option<String>,
    pub recipients: Option<Vec<String>>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub template_id: Option<String>,
}

/// Stored per-node fallback config (mirrors the builder panel fields).
/// `values` are the trigger's stored presets for its fallback template's
/// declared `{{refs}}` — the same node-level-preset doctrine the flow's
/// template node follows ([`crate::domain::flows::template_node_values`]).
///
/// `secrets` and `allowed_ips` are the door's own auth: any listed secret
/// requires a matching `x-notifi-secret` header, any listed rule (plain
/// IP or CIDR) requires the caller inside it. Empty lists keep the door
/// open — the documented, pre-auth default.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WebhookNodeConfig {
    pub template_id: Option<String>,
    pub channel: Option<String>,
    pub recipients: Vec<String>,
    pub values: std::collections::HashMap<String, String>,
    pub secrets: Vec<String>,
    pub allowed_ips: Vec<String>,
}

/// Tokens are 128-bit hex (see `newTriggerToken` on the frontend).
pub fn valid_trigger_token(token: &str) -> bool {
    token.len() == 32 && token.bytes().all(|b| b.is_ascii_hexdigit())
}

/// What an authorized ping left at the door, captured by the fire: the
/// caller's address, whitelisted headers, and the (parsed, re-serialized)
/// body. Stored per trigger token as the builder's "Incoming data" view.
#[derive(Debug, Clone, Default)]
pub struct InboundProbe {
    pub client_ip: Option<String>,
    pub content_type: Option<String>,
    pub user_agent: Option<String>,
    pub forwarded_for: Option<String>,
    pub body: Option<String>,
}

/// Body cap for probes — debug data, not a ledger.
pub const PROBE_BODY_CAP: usize = 16 * 1024;

/// Trim a probe body to [`PROBE_BODY_CAP`] bytes (char-boundary safe).
pub fn cap_probe_body(body: Option<String>) -> Option<String> {
    let body = body?;
    if body.len() <= PROBE_BODY_CAP {
        return Some(body);
    }
    let mut cut = PROBE_BODY_CAP;
    while !body.is_char_boundary(cut) {
        cut -= 1;
    }
    Some(body[..cut].to_string())
}

/// All `(token, node_id)` pairs for webhook trigger nodes in a flow
/// definition. Malformed nodes are skipped (validation flags them in
/// the builder).
pub fn extract_trigger_tokens(definition: &serde_json::Value) -> Vec<(String, String)> {
    let Some(nodes) = definition.get("nodes").and_then(|n| n.as_array()) else {
        return Vec::new();
    };
    nodes
        .iter()
        .filter_map(|n| {
            if n.get("type").and_then(|t| t.as_str()) != Some("trigger") {
                return None;
            }
            let id = n.get("id").and_then(|i| i.as_str())?;
            let data = n.get("data")?;
            if data.get("kind").and_then(|k| k.as_str()) != Some("webhook") {
                return None;
            }
            let token = data.get("token").and_then(|t| t.as_str())?;
            if !valid_trigger_token(token) {
                return None;
            }
            Some((token.to_string(), id.to_string()))
        })
        .collect()
}

/// Fallback config for one webhook trigger node, if it is one.
pub fn webhook_node_data(
    definition: &serde_json::Value,
    node_id: &str,
) -> Option<WebhookNodeConfig> {
    let nodes = definition.get("nodes").and_then(|n| n.as_array())?;
    let node = nodes.iter().find(|n| {
        n.get("id").and_then(|i| i.as_str()) == Some(node_id)
            && n.get("type").and_then(|t| t.as_str()) == Some("trigger")
    })?;
    let data = node.get("data")?;
    if data.get("kind").and_then(|k| k.as_str()) != Some("webhook") {
        return None;
    }
    let non_empty = |v: Option<&serde_json::Value>| {
        v.and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let string_list = |v: Option<&serde_json::Value>| -> Vec<String> {
        v.and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    };
    Some(WebhookNodeConfig {
        template_id: non_empty(data.get("templateId")),
        channel: non_empty(data.get("channel")),
        recipients: string_list(data.get("recipients")),
        values: crate::domain::flows::node_data_values(data),
        secrets: string_list(data.get("secrets")),
        allowed_ips: string_list(data.get("allowedIps")),
    })
}

/// Does the caller's address pass the node's IP rules? Entries are plain
/// IPs (`203.0.113.9`) or CIDR blocks (`10.0.0.0/8`); an unparseable
/// entry never passes — a malformed rule can't widen the door.
pub fn ip_allowed(allowed_ips: &[String], ip: std::net::IpAddr) -> bool {
    use std::str::FromStr;
    allowed_ips.iter().any(|rule| {
        if let Ok(net) = ipnet::IpNet::from_str(rule) {
            return net.contains(&ip);
        }
        std::net::IpAddr::from_str(rule.trim())
            .map(|parsed| parsed == ip)
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod inbound_entities_tests {
    use super::*;

    fn definition() -> serde_json::Value {
        serde_json::json!({
            "version": 1,
            "nodes": [
                {"id": "t1", "type": "trigger", "position": {"x": 0, "y": 0},
                 "data": {"kind": "webhook", "token": "aabbccddeeff00112233445566778899",
                          "templateId": "tpl_1", "channel": "email",
                          "recipients": ["u1", "u2"]}},
                {"id": "t2", "type": "trigger", "position": {"x": 0, "y": 0},
                 "data": {"kind": "event", "event": "notification.created"}},
                {"id": "t3", "type": "trigger", "position": {"x": 0, "y": 0},
                 "data": {"kind": "webhook"}},
            ],
            "edges": []
        })
    }

    #[test]
    fn extracts_only_well_formed_webhook_tokens() {
        assert_eq!(
            extract_trigger_tokens(&definition()),
            vec![(
                "aabbccddeeff00112233445566778899".to_string(),
                "t1".to_string()
            )]
        );
    }

    #[test]
    fn reads_node_fallback_config() {
        let config = webhook_node_data(&definition(), "t1").unwrap();
        assert_eq!(config.template_id.as_deref(), Some("tpl_1"));
        assert_eq!(config.channel.as_deref(), Some("email"));
        assert_eq!(config.recipients, vec!["u1".to_string(), "u2".to_string()]);
        assert!(config.values.is_empty(), "no values key reads as none");
        assert!(config.secrets.is_empty(), "no secrets key reads as none");
        assert!(
            config.allowed_ips.is_empty(),
            "no allowedIps key reads as none"
        );
        assert!(webhook_node_data(&definition(), "t2").is_none());
        assert!(webhook_node_data(&definition(), "nope").is_none());
    }

    #[test]
    fn reads_node_secrets_and_ip_rules() {
        let mut def = definition();
        def["nodes"][0]["data"]["secrets"] =
            serde_json::json!(["whsk_4420aaaa", " whsk_bbbb ", ""]);
        def["nodes"][0]["data"]["allowedIps"] =
            serde_json::json!(["203.0.113.9", " 10.0.0.0/8 ", ""]);
        let config = webhook_node_data(&def, "t1").unwrap();
        assert_eq!(
            config.secrets,
            vec!["whsk_4420aaaa".to_string(), "whsk_bbbb".to_string()]
        );
        assert_eq!(
            config.allowed_ips,
            vec!["203.0.113.9".to_string(), "10.0.0.0/8".to_string()]
        );

        let ip: std::net::IpAddr = "10.1.2.3".parse().unwrap();
        assert!(ip_allowed(&config.allowed_ips, ip), "inside the CIDR");
        let flip: std::net::IpAddr = "198.51.100.1".parse().unwrap();
        assert!(!ip_allowed(&config.allowed_ips, flip), "outside every rule");
        assert!(
            !ip_allowed(&["garbage".to_string()], ip),
            "bad rule never passes"
        );
        assert!(!ip_allowed(&[], ip), "empty list passes nobody");
    }

    #[test]
    fn reads_node_value_presets() {
        let mut def = definition();
        def["nodes"][0]["data"]["values"] = serde_json::json!({
            "code": "  4420 ", "gone": "", "bad": 7
        });
        let config = webhook_node_data(&def, "t1").unwrap();
        assert_eq!(
            config.values.len(),
            1,
            "blank and non-string entries are skipped"
        );
        assert_eq!(config.values["code"], "4420", "presets are trimmed");
        assert!(!config.values.contains_key("gone"));
        assert!(!config.values.contains_key("bad"));
    }

    #[test]
    fn caps_probe_bodies_on_char_boundaries() {
        assert!(cap_probe_body(None).is_none());
        assert_eq!(
            cap_probe_body(Some("small".into())).as_deref(),
            Some("small")
        );
        let big = "é".repeat(PROBE_BODY_CAP); // 2 bytes each — over the cap
        let cut = cap_probe_body(Some(big)).unwrap();
        assert!(cut.len() <= PROBE_BODY_CAP, "cut inside the cap");
        assert!(cut.is_char_boundary(cut.len()), "no torn character");
        assert_eq!(cut.chars().count(), PROBE_BODY_CAP / 2);
    }
}
