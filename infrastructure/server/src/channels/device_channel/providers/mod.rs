pub mod android;
pub mod apns;
pub mod linux;
pub mod web;
pub mod window;
pub mod window_webhook;
pub mod wns;

use futures::future::BoxFuture;

/// Window's inner sender contract: the flavor (webhook or WNS) decides
/// how `target` (webhook URL or WNS channel URI) is reached.
pub trait WindowSender: Send + Sync {
    fn send(&self, target: &str, title: &str, body: &str)
    -> BoxFuture<'static, Result<(), String>>;
}

/// The destination the window flavor sends to (URL or channel URI).
pub struct WindowMessage {
    pub target: String,
    pub title: String,
    pub body: String,
}

/// All supported device providers. THIS IS THE SOURCE OF TRUTH.
///
/// Unlike chat/sms, device providers differ in their destination model
/// (native push token, self-hosted endpoint, WNS channel URI, web-push
/// subscription), so the family dispatches by variant instead of forcing
/// one fake-uniform message type; the `ios`/`macos` flavors share the
/// APNs controller in [`apns`].
#[derive(Debug, Clone)]
pub enum DeviceProvider {
    Android(android::AndroidConfig),
    Ios(apns::IosConfig),
    Macos(apns::MacosConfig),
    Linux(linux::LinuxConfig),
    Window(window::WindowConfig),
    Web(web::WebConfig),
}

impl DeviceProvider {
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::Android(_) => "android",
            Self::Ios(_) => "ios",
            Self::Macos(_) => "macos",
            Self::Linux(_) => "linux",
            Self::Window(_) => "window",
            Self::Web(_) => "web",
        }
    }
}
