//! Device channel providers, grouped per destination.
//!
//! The OS is the destination; vendors live inside their destination's
//! folder (`android/fcm.rs`, `apple/apns.rs`, …). Destination shapes
//! genuinely differ (push token / endpoint / WNS channel URI /
//! web-push subscription), so the family dispatches by destination
//! instead of forcing one uniform message type.

pub mod android;
pub mod apple;
pub mod linux;
pub mod web;
pub mod window;

/// All supported device providers. THIS IS THE SOURCE OF TRUTH.
///
/// Each variant is a destination; the inner enum (same unit's `Provider`)
/// names the vendor serving it.
#[derive(Debug, Clone)]
pub enum DeviceProvider {
    Android(android::Provider),
    Apple(apple::Provider),
    Linux(linux::Provider),
    Web(web::Provider),
    Window(window::Provider),
}

impl DeviceProvider {
    /// The destination half: which OS flavor receives the message.
    pub fn destination_name(&self) -> &'static str {
        match self {
            Self::Android(_) => "android",
            Self::Apple(_) => "apple",
            Self::Linux(_) => "linux",
            Self::Web(_) => "web",
            Self::Window(_) => "window",
        }
    }

    /// The vendor half: which delivery service carries it inside that
    /// destination.
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::Android(p) => p.provider_name(),
            Self::Apple(p) => p.provider_name(),
            Self::Linux(p) => p.provider_name(),
            Self::Web(p) => p.provider_name(),
            Self::Window(p) => p.provider_name(),
        }
    }
}

#[cfg(test)]
mod device_dest_tests {
    use super::*;

    #[test]
    fn destination_names_cover_all_family_members() {
        let names: Vec<&str> = [
            DeviceProvider::Android(android::Provider::Fcm(fcm_like_config())).destination_name(),
            DeviceProvider::Apple(apple::Provider::Apns(apns_like_config())).destination_name(),
            DeviceProvider::Linux(linux::Provider::Endpoint(
                linux::endpoint::LinuxConfig::default(),
            ))
            .destination_name(),
            DeviceProvider::Web(web::Provider::WebPush(web_push_like_config())).destination_name(),
            DeviceProvider::Window(window::Provider::Webhook).destination_name(),
        ]
        .to_vec();
        assert_eq!(names, ["android", "apple", "linux", "web", "window"]);
    }

    fn fcm_like_config() -> android::fcm::FcmConfig {
        android::fcm::FcmConfig {
            service_account_path: std::path::PathBuf::from("/x/sa.json"),
        }
    }

    fn apns_like_config() -> apple::apns::IosConfig {
        apple::apns::IosConfig {
            key_id: "k".to_string(),
            team_id: "t".to_string(),
            bundle_id: "b".to_string(),
            p8_key_path: std::path::PathBuf::from("/k.p8"),
            sandbox: false,
        }
    }

    fn web_push_like_config() -> web::web_push::WebConfig {
        web::web_push::WebConfig {
            vapid_public_key: "p".to_string(),
            vapid_private_key: "k".to_string(),
            contact_email: "a@b.c".to_string(),
        }
    }
}
