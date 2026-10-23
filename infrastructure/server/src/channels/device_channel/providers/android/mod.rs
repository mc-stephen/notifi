//! Android destination — everything that can push to an Android device.
//!
//! `provider_name()` per file; the enum here is the destination's source
//! of truth for which vendor serves it. Third-party vendors beyond FCM
//! have config shells; their controllers fail closed until wired.

pub mod fcm;
pub mod onesignal;
pub mod pushy;

/// The Android destination's own message shape: an FCM-style
/// registration token plus the notification copy.
pub struct AndroidMessage {
    pub token: String,
    pub title: String,
    pub body: String,
}

/// All supported Android push providers. THIS IS THE SOURCE OF TRUTH.
#[derive(Debug, Clone)]
pub enum Provider {
    Fcm(fcm::FcmConfig),
    Pushy(pushy::PushyConfig),
    OneSignal(onesignal::OneSignalConfig),
}

impl Provider {
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::Fcm(_) => "fcm",
            Self::Pushy(_) => "pushy",
            Self::OneSignal(_) => "onesignal",
        }
    }
}

#[cfg(test)]
mod android_dest_tests {
    use super::*;

    #[test]
    fn provider_names_cover_all_variants() {
        let fcm = Provider::Fcm(fcm::FcmConfig {
            service_account_path: std::path::PathBuf::from("/x/sa.json"),
        });
        assert_eq!(fcm.provider_name(), "fcm");
        let pushy = Provider::Pushy(pushy::PushyConfig {
            api_key: "k".to_string(),
            app_id: None,
        });
        assert_eq!(pushy.provider_name(), "pushy");
        let onesignal = Provider::OneSignal(onesignal::OneSignalConfig {
            app_id: "a".to_string(),
            api_key: "k".to_string(),
            rest_api_key: None,
        });
        assert_eq!(onesignal.provider_name(), "onesignal");
    }
}
