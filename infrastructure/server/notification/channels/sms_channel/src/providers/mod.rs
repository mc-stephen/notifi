pub mod twilio;
pub mod local_nigeria;

pub use twilio::TwilioProvider;
pub use local_nigeria::LocalNigeriaProvider;
use futures::future::BoxFuture;

pub trait SmsSender: Send + Sync {
    fn send(&self, to: &str, text: &str) -> BoxFuture<'static, Result<(), String>>;
}
