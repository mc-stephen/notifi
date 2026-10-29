//! Linux destination — the hosted Notifi Gateway.
//!
//! Notifi hosts the listening path: sends queue onto the user's secret
//! route (a topic name they pick + Notifi's generated key), and their
//! Linux app polls `GET /gateway/linux/{topic}?key=...` to read and
//! clear it. Desktop Linux has no vendor-shaped push worth an account,
//! so the gateway keeps the whole story inside Notifi (route surface:
//! `src/api/routes/gateway.rs`; queue port: `src/ports/gateway.rs`).
//! The old self-hosted webhook endpoint flavor was cut outright in the
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
/// by the device family's golden test. `docs_url` stays unset until the
/// docs app hosts the gateway explainer (the card must never 404).
pub fn vendors() -> Vec<super::VendorEntry> {
    vec![super::VendorEntry {
        id: "gateway",
        name: "Notifi Gateway",
        platforms: &["linux"],
        icon_url: None,
        docs_url: None,
        fields: vec![
            super::VendorField {
                key: "topic",
                label: "Route topic",
                ty: super::FieldType::Text,
                required: true,
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
