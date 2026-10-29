//! Linux destination — the hosted Notifi Gateway flavor.
//!
//! Notifi hosts the listening path: sends queue onto the user's secret
//! route (topic + generated key), and their Linux app polls
//! `GET /gateway/linux/{topic}?key=...` to read and clear it — no vendor
//! account, no listener for the user to run. Both route halves come from
//! the saved recipe, so the queue needs no registry of its own. Route
//! surface: `src/api/routes/gateway.rs`; queue port:
//! `src/ports/gateway.rs`.

use serde::Deserialize;

/// The saved gateway recipe (channel `linux`, provider `gateway`).
///
/// The connect form prefills a generated secret and offers a regenerate
/// button; `mode` is the user's display pick (`polling` = long-poll holds
/// the request for instant landings, `https` = plain checks on the app's
/// own cadence) — advisory, because the poll hold is the app's own choice
/// per request (`?wait=N`), not saved state.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct GatewayRecipe {
    /// The secret-route topic the user picked (the URL path under
    /// `/gateway/linux/`).
    pub topic: String,
    /// The generated route secret (route both halves must match).
    #[serde(default)]
    pub secret_key: String,
    /// User's pick for the connect summary (`polling` or `https`).
    #[serde(default)]
    pub mode: Option<String>,
}

impl GatewayRecipe {
    /// True when the recipe holds both route halves (topic + secret).
    pub fn carries_credentials(&self) -> bool {
        !self.topic.trim().is_empty() && !self.secret_key.trim().is_empty()
    }
}
