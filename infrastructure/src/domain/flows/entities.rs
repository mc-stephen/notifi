//! Flow definitions: notification routing graphs.
//!
//! Flows v2: every project flow lives server-side. There is no stored
//! "default" flag — a flow qualifies for sends by carrying a
//! `notification.created` trigger node (see [`handles_event`]).
//! Framework-free by rule.

/// A stored flow definition.
#[derive(Debug, Clone)]
pub struct Flow {
    pub id: String,
    pub project_id: String,
    pub name: String,
    /// Frontend `FlowDoc` shape (`{version, nodes, edges}`), stored as-is.
    pub definition: serde_json::Value,
    pub version: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// A provider step extracted from a flow definition for execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderStep {
    pub channel_id: String,
    pub provider_id: String,
    pub priority: i64,
}

/// Whether a flow definition carries a trigger node for an event.
/// This is what makes a flow sendable — no stored flag involved.
pub fn handles_event(definition: &serde_json::Value, event: &str) -> bool {
    definition
        .get("nodes")
        .and_then(|n| n.as_array())
        .is_some_and(|nodes| {
            nodes.iter().any(|n| {
                n.get("type").and_then(|t| t.as_str()) == Some("trigger")
                    && n.get("data")
                        .and_then(|d| d.get("event"))
                        .and_then(|e| e.as_str())
                        == Some(event)
            })
        })
}

/// Collects provider steps for one channel, ordered by priority ascending.
/// Malformed nodes are skipped (the UI validator flags them separately).
pub fn provider_chain(definition: &serde_json::Value, channel: &str) -> Vec<ProviderStep> {
    let Some(nodes) = definition.get("nodes").and_then(|n| n.as_array()) else {
        return Vec::new();
    };
    let mut steps: Vec<ProviderStep> = nodes
        .iter()
        .filter(|n| n.get("type").and_then(|t| t.as_str()) == Some("provider"))
        .filter_map(|n| {
            let data = n.get("data")?;
            let node_channel = data.get("channelId").and_then(|c| c.as_str())?;
            if node_channel != channel {
                return None;
            }
            Some(ProviderStep {
                channel_id: node_channel.to_string(),
                provider_id: data
                    .get("providerId")
                    .and_then(|p| p.as_str())
                    .unwrap_or_default()
                    .to_string(),
                priority: data
                    .get("priority")
                    .and_then(|p| p.as_i64())
                    .unwrap_or(i64::MAX),
            })
        })
        .filter(|s| !s.provider_id.is_empty())
        .collect();
    steps.sort_by_key(|s| s.priority);
    steps
}

#[cfg(test)]
mod flow_tests {
    use super::*;

    fn sample_definition() -> serde_json::Value {
        serde_json::json!({
            "version": 1,
            "nodes": [
                {
                    "id": "nd_trigger",
                    "type": "trigger",
                    "position": { "x": 80, "y": 200 },
                    "data": { "kind": "event", "event": "notification.created" },
                },
                {
                    "id": "nd_email_smtp",
                    "type": "provider",
                    "position": { "x": 420, "y": 40 },
                    "data": { "channelId": "email", "providerId": "smtp", "priority": 1 },
                },
                {
                    "id": "nd_push_fcm",
                    "type": "provider",
                    "position": { "x": 420, "y": 280 },
                    "data": { "channelId": "push", "providerId": "fcm", "priority": 1 },
                },
                {
                    "id": "nd_push_apns",
                    "type": "provider",
                    "position": { "x": 420, "y": 400 },
                    "data": { "channelId": "push", "providerId": "apns", "priority": 2 },
                },
            ],
            "edges": [],
        })
    }

    #[test]
    fn trigger_handles_notification_created() {
        let def = sample_definition();
        assert!(handles_event(&def, "notification.created"));
        assert!(!handles_event(&def, "notification.failed"));
        assert!(!handles_event(
            &serde_json::json!({}),
            "notification.created"
        ));
    }

    #[test]
    fn provider_chain_per_channel() {
        let def = sample_definition();
        let push = provider_chain(&def, "push");
        assert_eq!(push.len(), 2);
        assert_eq!(push[0].provider_id, "fcm");
        assert_eq!(push[1].provider_id, "apns");
        assert_eq!(provider_chain(&def, "email").len(), 1);
        assert!(provider_chain(&def, "nope").is_empty());
    }

    #[test]
    fn malformed_nodes_are_skipped() {
        let def = serde_json::json!({ "nodes": [{ "type": "provider", "data": {} }] });
        assert!(provider_chain(&def, "email").is_empty());
        assert!(provider_chain(&serde_json::json!({}), "email").is_empty());
    }
}
