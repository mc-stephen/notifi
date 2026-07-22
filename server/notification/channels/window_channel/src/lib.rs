pub mod config;
pub mod providers;

pub use config::WindowConfig;
use providers::{webhook::WebhookProvider, wns::WnsProvider, WindowSender};

pub struct WindowMessage {
    pub target: String, // Webhook URL or WNS URI
    pub title: String,
    pub body: String,
}

pub struct WindowProvider {
    sender: Box<dyn WindowSender>,
}

impl WindowProvider {
    pub fn new(config: WindowConfig) -> Self {
        let sender: Box<dyn WindowSender> = match config {
            WindowConfig::Webhook {} => {
                Box::new(WebhookProvider::new())
            }
            WindowConfig::Wns { client_id, client_secret, package_sid } => {
                Box::new(WnsProvider::new(client_id, client_secret, package_sid))
            }
        };
        Self { sender }
    }

    pub async fn send_notification(&self, msg: &WindowMessage) -> Result<(), String> {
        self.sender.send(&msg.target, &msg.title, &msg.body).await
    }
}
