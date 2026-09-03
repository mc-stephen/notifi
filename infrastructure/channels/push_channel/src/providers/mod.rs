pub mod fcm;
pub mod apns;
pub mod onesignal;
pub mod pushy;
pub mod braze_airship;

/// All supported push providers. THIS IS THE SOURCE OF TRUTH.
#[derive(Debug, Clone)]
pub enum PushProvider {
    Fcm(fcm::FcmConfig),
    Apns(apns::ApnsConfig),
    OneSignal(onesignal::OneSignalConfig),
    Pushy(pushy::PushyConfig),
    BrazeAirship(braze_airship::BrazeAirshipConfig),
}

impl PushProvider {
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::Fcm(_) => "fcm",
            Self::Apns(_) => "apns",
            Self::OneSignal(_) => "onesignal",
            Self::Pushy(_) => "pushy",
            Self::BrazeAirship(_) => "braze_airship",
        }
    }
}
