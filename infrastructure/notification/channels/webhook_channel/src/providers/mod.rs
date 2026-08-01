pub mod function;
pub mod relay;

use futures::future::BoxFuture;

pub trait WebhookSender: Send + Sync {
    fn send(&self, title: &str, body: &str, brand: &str) -> BoxFuture<'static, Result<(), String>>;
}
