pub mod config;
pub mod providers;

pub use config::WebhookConfig;
use providers::{WebhookSender, function::FunctionProvider, relay::RelayProvider};

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
