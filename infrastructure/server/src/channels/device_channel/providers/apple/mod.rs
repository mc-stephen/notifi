//! Apple destination — everything that can push to an iPhone or Mac.
//!
//! Both OSes ride the same APNs signing flow; the flavor lives in the
//! device configs (bundle ids differ), not in a separate controller.
//! Third-party vendors have config shells; their controllers fail
//! closed until wired.

pub mod apns;
pub mod onesignal;

/// All supported Apple push providers. THIS IS THE SOURCE OF TRUTH.
///
/// `Apns` and `ApnsMacos` are the same vendor against a different device
/// flavor (iPhone bundle vs Mac bundle); each carries its own APNs config.
#[derive(Debug, Clone)]
pub enum Provider {
    Apns(apns::IosConfig),
    ApnsMacos(apns::MacosConfig),
    OneSignal(onesignal::OneSignalConfig),
}

impl Provider {
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::Apns(_) => "apns",
            Self::ApnsMacos(_) => "apns_macos",
            Self::OneSignal(_) => "onesignal",
        }
    }
}

#[cfg(test)]
mod apple_dest_tests {
    use super::*;

    #[test]
    fn provider_names_cover_all_variants() {
        let apns = Provider::Apns(apns::IosConfig {
            key_id: "k".to_string(),
            team_id: "t".to_string(),
            bundle_id: "b".to_string(),
            p8_key_path: std::path::PathBuf::from("/k.p8"),
            sandbox: false,
        });
        assert_eq!(apns.provider_name(), "apns");
        let macos = Provider::ApnsMacos(apns::MacosConfig {
            key_id: "k".to_string(),
            team_id: "t".to_string(),
            bundle_id: "b".to_string(),
            p8_key_path: std::path::PathBuf::from("/k.p8"),
            sandbox: false,
        });
        assert_eq!(macos.provider_name(), "apns_macos");
    }
}
