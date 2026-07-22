pub mod relay;
pub mod function;

use futures::future::BoxFuture;

pub trait WebhookSender: Send + Sync {
    fn send(&self, title: &str, body: &str, brand: &str) -> BoxFuture<'static, Result<(), String>>;
}
