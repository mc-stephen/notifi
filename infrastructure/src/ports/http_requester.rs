//! Outbound HTTP for flow HTTP Request steps.
//!
//! Framework-free by rule: plain string types only. Timeouts are applied
//! by callers (matching the provider attempt budget).

use std::future::Future;
use std::pin::Pin;

/// Boxed future returned by the request method.
pub type BoxHttpFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// One outbound call.
#[derive(Debug, Clone)]
pub struct HttpCall {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// What came back (failures surface as `Err(String)`).
#[derive(Debug, Clone)]
pub struct HttpResult {
    pub status: u16,
    pub ok: bool,
}

pub trait HttpRequester: Send + Sync {
    fn request(&self, call: &HttpCall) -> BoxHttpFut<'_, Result<HttpResult, String>>;
}
