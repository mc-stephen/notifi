//! Webhook entities: stored rows plus normalization helpers.
//!
//! A webhook is a flow **owner**, not a flow client: creating a webhook
//! creates its flow (named after the webhook, seeded with a locked
//! webhook trigger node carrying a fresh 128-bit token for the inbound
//! fire path) and deleting the webhook deletes that flow. `archived_at`
//! is the non-destructive retirement path — archived webhooks keep their
//! flow bound and restorable.

use chrono::{DateTime, Utc};

/// A stored webhook. `flow_name` rides along via JOIN so list views never
/// issue a second query.
#[derive(Debug, Clone)]
pub struct Webhook {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub flow_id: String,
    pub flow_name: String,
    pub events: Vec<String>,
    pub active: bool,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input for creating a webhook. The flow is created by the service —
/// callers never supply or assign one. `active` starts true.
#[derive(Debug, Clone)]
pub struct WebhookInput {
    pub name: String,
    pub description: String,
    pub events: Vec<String>,
}

/// Partial update. Absent fields keep the current value.
#[derive(Debug, Clone, Default)]
pub struct WebhookPatch {
    pub name: Option<String>,
    pub description: Option<String>,
    /// Replacement list for the trigger's event names (metadata for the
    /// locked trigger node).
    pub events: Option<Vec<String>>,
    pub active: Option<bool>,
    /// `true` archives (stamps `archived_at`), `false` restores.
    pub archived: Option<bool>,
}

/// Trims event names; drops empties and duplicates while keeping first
/// occurrence order.
pub fn normalize_events(events: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for e in events {
        let e = e.trim();
        if !e.is_empty() && seen.insert(e.to_string()) {
            out.push(e.to_string());
        }
    }
    out
}

#[cfg(test)]
mod webhook_entities_tests {
    use super::*;

    #[test]
    fn events_trim_and_dedupe() {
        assert_eq!(
            normalize_events(&[
                " notification.created ".to_string(),
                "notification.created".to_string(),
                "".to_string(),
                "notification.failed".to_string(),
            ]),
            vec![
                "notification.created".to_string(),
                "notification.failed".to_string()
            ]
        );
        assert!(normalize_events(&["  ".to_string()]).is_empty());
    }
}
