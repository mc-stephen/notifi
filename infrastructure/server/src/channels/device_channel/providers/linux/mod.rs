//! Linux destination — the hosted Notifi Gateway.
//!
//! Notifi hosts the listening path: sends queue onto the user's secret
//! route (a topic + secret pair Notifi generates at connect — the form
//! asks nothing), and their Linux app polls
//! `GET /gateway/linux/{topic}?key=...` to read and clear it. Desktop
//! Linux has no vendor-shaped push worth an account, so the gateway keeps
//! the whole story inside Notifi (route surface:
//! `src/api/routes/gateway.rs`; queue port: `src/ports/gateway.rs`). The
//! old self-hosted webhook endpoint flavor was cut outright in the
//! gateway's landing (dev stage, no life data — same call as
//! `apns_macos`).

pub mod gateway;

pub use gateway::GatewayRecipe;

/// All supported Linux providers. THIS IS THE SOURCE OF TRUTH.
#[derive(Debug, Clone)]
pub enum Provider {
    Gateway(GatewayRecipe),
}

impl Provider {
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::Gateway(_) => "gateway",
        }
    }
}

/// The destination's registry listing — kept in sync with `Provider`
/// by the device family's golden test. The route is static: both halves
/// (topic + secret) are generated at connect time, so the form asks
/// nothing — `topic` is a hidden field the recipe keeps (and the enrolled
/// URL shows), `secret_key` is prefilled readonly with a regenerate
/// button, and the mode dropdown is the one real choice. The docs link
/// points at our own explainer post
/// (`app/documentation/.../channels/linux-gateway.mdx`), not a vendor
/// page — there is no vendor.
pub fn vendors() -> Vec<super::VendorEntry> {
    vec![super::VendorEntry {
        id: "gateway",
        name: "Notifi Gateway",
        platforms: &["linux"],
        icon_url: None,
        docs_url: Some("https://docs.notifi.dev/channels/linux-gateway"),
        fields: vec![
            super::VendorField {
                key: "topic",
                label: "Route topic",
                ty: super::FieldType::Hidden,
                required: false,
            },
            super::VendorField {
                key: "secret_key",
                label: "Route secret",
                ty: super::FieldType::Text,
                required: true,
            },
            super::VendorField {
                key: "mode",
                label: "Delivery mode",
                ty: super::FieldType::Select(&["polling", "https"]),
                required: true,
            },
        ],
    }]
}

#[cfg(test)]
mod linux_dest_tests {
    use super::*;

    #[test]
    fn family_card_names_the_gateway() {
        let p = Provider::Gateway(GatewayRecipe::default());
        assert_eq!(p.provider_name(), "gateway");
    }
}
