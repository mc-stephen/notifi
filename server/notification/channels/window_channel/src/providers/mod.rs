pub mod webhook;
pub mod wns;

use futures::future::BoxFuture;

pub trait WindowSender: Send + Sync {
    // Target is either Webhook URL or WNS channel URI
    fn send(&self, target: &str, title: &str, body: &str) -> BoxFuture<'static, Result<(), String>>;
}
