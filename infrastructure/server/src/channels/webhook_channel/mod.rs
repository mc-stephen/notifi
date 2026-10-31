pub mod config;
pub mod function;
pub mod relay;

pub use config::WebhookConfig;
use function::FunctionProvider;
use relay::RelayProvider;

use futures::future::BoxFuture;

pub trait WebhookSender: Send + Sync {
    fn send(
        &self,
        title: &str,
        body: &str,
        project: &str,
    ) -> BoxFuture<'static, Result<(), String>>;
}

pub struct WebhookMessage {
    pub title: String,
    pub body: String,
}

pub struct WebhookProvider {
    sender: Box<dyn WebhookSender>,
    project: String,
}

impl WebhookProvider {
    pub fn new(config: WebhookConfig, project: String) -> Self {
        let sender: Box<dyn WebhookSender> = match config {
            WebhookConfig::Relay {
                target_url,
                method,
                headers,
                payload_template,
            } => Box::new(RelayProvider::new(
                target_url,
                method,
                headers,
                payload_template,
            )),
            WebhookConfig::Function { function_id } => Box::new(FunctionProvider::new(function_id)),
        };
        Self { sender, project }
    }

    pub async fn send_notification(&self, msg: &WebhookMessage) -> Result<(), String> {
        self.sender.send(&msg.title, &msg.body, &self.project).await
    }
}
