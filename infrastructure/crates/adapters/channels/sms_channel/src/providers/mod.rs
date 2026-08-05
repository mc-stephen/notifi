pub mod local_nigeria;
pub mod twilio;

use futures::future::BoxFuture;
pub use local_nigeria::LocalNigeriaProvider;
pub use twilio::TwilioProvider;

pub trait SmsSender: Send + Sync {
    fn send(&self, to: &str, text: &str) -> BoxFuture<'static, Result<(), String>>;
}
