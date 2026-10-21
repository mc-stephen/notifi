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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WebhookNodeConfig {
    pub template_id: Option<String>,
    pub channel: Option<String>,
    pub recipients: Vec<String>,
}

/// Tokens are 128-bit hex (see `newTriggerToken` on the frontend).
pub fn valid_trigger_token(token: &str) -> bool {
    token.len() == 32 && token.bytes().all(|b| b.is_ascii_hexdigit())
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
    Some(WebhookNodeConfig {
        template_id: non_empty(data.get("templateId")),
        channel: non_empty(data.get("channel")),
        recipients: data
            .get("recipients")
            .and_then(|r| r.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default(),
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
        assert!(webhook_node_data(&definition(), "t2").is_none());
        assert!(webhook_node_data(&definition(), "nope").is_none());
    }
}
