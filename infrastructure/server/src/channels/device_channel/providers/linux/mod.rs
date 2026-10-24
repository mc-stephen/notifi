//! Linux destination — currently a single self-hosted endpoint flavor.
//!
//! Desktop Linux has no vendor-shaped push; delivery rides a webhook
//! endpoint supplied per message, optionally with auth headers. The
//! enum keeps the family shape uniform for when a real flavor lands.

pub mod endpoint;

/// All supported Linux providers. THIS IS THE SOURCE OF TRUTH.
#[derive(Debug, Clone)]
pub enum Provider {
    Endpoint(endpoint::LinuxConfig),
}

impl Provider {
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::Endpoint(_) => "endpoint",
        }
    }
}

/// The destination's registry listing — kept in sync with `Provider`
/// by the family's golden test. Nothing to configure: the receiver
/// endpoint is the per-device target, not provider state.
pub fn vendors() -> Vec<super::VendorEntry> {
    vec![super::VendorEntry {
        id: "endpoint",
        name: "Self-hosted endpoint",
        platforms: &["linux"],
        icon_url: None,
        docs_url: Some("https://specifications.freedesktop.org/notification-spec/latest/"),
        fields: vec![],
    }]
}

#[cfg(test)]
mod linux_dest_tests {
    use super::*;

    #[test]
    fn provider_name_is_endpoint() {
        let p = Provider::Endpoint(endpoint::LinuxConfig::default());
        assert_eq!(p.provider_name(), "endpoint");
    }
}
