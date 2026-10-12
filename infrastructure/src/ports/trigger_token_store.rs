//! Lookup port for inbound webhook trigger tokens.
//!
//! The flow definition is the source of truth (tokens live on trigger
//! nodes); this table is a lookup index synced on every flow save.

use std::future::Future;
use std::pin::Pin;

use crate::ports::auth_store::StoreError;

/// Boxed future returned by every port method.
pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A token row with its flow's definition joined in (single-query fire).
#[derive(Debug, Clone)]
pub struct TriggerTokenRecord {
    pub token: String,
    pub flow_id: String,
    pub node_id: String,
    pub project_id: String,
    pub flow_definition: serde_json::Value,
}

pub trait TriggerTokenStore: Send + Sync {
    /// Replace every token row for a flow with the given pairs
    /// (`(token, node_id)`), deleting stale ones. `definition` is the
    /// flow snapshot for stores that can't JOIN (in-memory fakes).
    fn replace_flow_tokens(
        &self,
        flow_id: &str,
        project_id: &str,
        definition: &serde_json::Value,
        pairs: &[(String, String)],
    ) -> BoxFut<'_, Result<(), StoreError>>;

    /// One token with its flow definition, if the token exists.
    fn lookup(&self, token: &str) -> BoxFut<'_, Result<Option<TriggerTokenRecord>, StoreError>>;
}
