//! HTTP Request step collection + firing for flow runs.
//!
//! v1 semantics are deliberately simple: every `http-request` node in the
//! definition fires once per run, regardless of edges or conditions. Full
//! edge-traversal evaluation (including If-on-result branching) is M-flow
//! work; until then failures are logged server-side, never fail the send,
//! and never appear in deliveries.

use std::sync::Arc;
use std::time::Duration;

use crate::ports::http_requester::{HttpCall, HttpRequester};

/// Per-step network budget (shorter than provider attempts: side quests
/// must not stall the send they ride along with).
pub const HTTP_STEP_TIMEOUT: Duration = Duration::from_secs(10);

/// One configured HTTP step extracted from a flow definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpStep {
    pub node_id: String,
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// Outcome of firing one step (logged by callers, never fatal).
#[derive(Debug, Clone)]
pub struct HttpStepOutcome {
    pub node_id: String,
    pub ok: bool,
    pub status: Option<u16>,
    pub error: Option<String>,
}

fn valid_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

/// Collects well-formed HTTP steps; malformed nodes are skipped (the UI
/// validator flags them separately).
pub fn collect_http_steps(definition: &serde_json::Value) -> Vec<HttpStep> {
    let Some(nodes) = definition.get("nodes").and_then(|n| n.as_array()) else {
        return Vec::new();
    };
    nodes
        .iter()
        .filter(|n| n.get("type").and_then(|t| t.as_str()) == Some("http-request"))
        .filter_map(|n| {
            let id = n.get("id").and_then(|i| i.as_str())?;
            let data = n.get("data")?;
            let url = data.get("url").and_then(|u| u.as_str()).unwrap_or_default();
            if !valid_url(url) {
                return None;
            }
            let method = data
                .get("method")
                .and_then(|m| m.as_str())
                .unwrap_or("POST")
                .to_string();
            let headers = data
                .get("headers")
                .and_then(|h| h.as_array())
                .map(|hs| {
                    hs.iter()
                        .filter_map(|h| {
                            let key = h.get("key").and_then(|k| k.as_str())?;
                            let value = h.get("value").and_then(|v| v.as_str()).unwrap_or_default();
                            if key.trim().is_empty() {
                                return None;
                            }
                            Some((key.to_string(), value.to_string()))
                        })
                        .collect()
                })
                .unwrap_or_default();
            let body = data
                .get("body")
                .and_then(|b| b.as_str())
                .unwrap_or_default()
                .to_string();
            Some(HttpStep {
                node_id: id.to_string(),
                method,
                url: url.to_string(),
                headers,
                body,
            })
        })
        .collect()
}

/// Fires every step once, in definition order. Individual failures are
/// captured in the outcome, never raised.
pub async fn fire_http_steps(
    requester: &Arc<dyn HttpRequester>,
    steps: &[HttpStep],
) -> Vec<HttpStepOutcome> {
    let mut outcomes = Vec::with_capacity(steps.len());
    for step in steps {
        let call = HttpCall {
            method: step.method.clone(),
            url: step.url.clone(),
            headers: step.headers.clone(),
            body: step.body.clone(),
        };
        match tokio::time::timeout(HTTP_STEP_TIMEOUT, requester.request(&call)).await {
            Ok(Ok(result)) => outcomes.push(HttpStepOutcome {
                node_id: step.node_id.clone(),
                ok: result.ok,
                status: Some(result.status),
                error: None,
            }),
            Ok(Err(e)) => outcomes.push(HttpStepOutcome {
                node_id: step.node_id.clone(),
                ok: false,
                status: None,
                error: Some(e),
            }),
            Err(_) => outcomes.push(HttpStepOutcome {
                node_id: step.node_id.clone(),
                ok: false,
                status: None,
                error: Some("http step timed out".to_string()),
            }),
        }
    }
    outcomes
}

#[cfg(test)]
mod http_steps_tests {
    use super::*;
    use crate::ports::http_requester::{BoxHttpFut, HttpResult};

    struct ScriptedRequester {
        fail: bool,
    }

    impl HttpRequester for ScriptedRequester {
        fn request(&self, call: &HttpCall) -> BoxHttpFut<'_, Result<HttpResult, String>> {
            assert!(call.url.starts_with("https://"));
            let fail = self.fail;
            Box::pin(async move {
                if fail {
                    Err("boom".to_string())
                } else {
                    Ok(HttpResult {
                        status: 200,
                        ok: true,
                    })
                }
            })
        }
    }

    fn definition() -> serde_json::Value {
        serde_json::json!({
            "version": 1,
            "nodes": [
                {"id": "h1", "type": "http-request", "position": {"x": 0, "y": 0},
                 "data": {"url": "https://api.example.com/hook", "method": "POST",
                          "headers": [{"key": "X-A", "value": "1"}, {"key": "", "value": "x"}],
                          "body": "{}"}},
                {"id": "h2", "type": "http-request", "position": {"x": 0, "y": 0},
                 "data": {"url": "not-a-url", "method": "POST", "headers": [], "body": ""}},
                {"id": "p1", "type": "provider", "position": {"x": 0, "y": 0}, "data": {}},
            ],
            "edges": []
        })
    }

    #[test]
    fn collects_only_well_formed_http_steps() {
        let steps = collect_http_steps(&definition());
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].node_id, "h1");
        assert_eq!(steps[0].headers, vec![("X-A".to_string(), "1".to_string())]);
    }

    #[tokio::test]
    async fn firing_captures_outcomes_without_raising() {
        let steps = collect_http_steps(&definition());
        let ok_requester: Arc<dyn HttpRequester> = Arc::new(ScriptedRequester { fail: false });
        let outcomes = fire_http_steps(&ok_requester, &steps).await;
        assert_eq!(outcomes.len(), 1);
        assert!(outcomes[0].ok);

        let fail_requester: Arc<dyn HttpRequester> = Arc::new(ScriptedRequester { fail: true });
        let outcomes = fire_http_steps(&fail_requester, &steps).await;
        assert!(!outcomes[0].ok);
        assert_eq!(outcomes[0].error.as_deref(), Some("boom"));
    }
}
