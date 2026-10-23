//! Web destination — browser push via the VAPID-signed web-push flow.

pub mod web_push;

/// All supported web (browser push) providers. THIS IS THE SOURCE OF TRUTH.
#[derive(Debug, Clone)]
pub enum Provider {
    WebPush(web_push::WebConfig),
}

impl Provider {
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::WebPush(_) => "web_push",
        }
    }
}

#[cfg(test)]
mod web_dest_tests {
    use super::*;

    #[test]
    fn provider_name_is_web_push() {
        let p = Provider::WebPush(web_push::WebConfig {
            vapid_public_key: "p".to_string(),
            vapid_private_key: "k".to_string(),
            contact_email: "a@b.c".to_string(),
        });
        assert_eq!(p.provider_name(), "web_push");
    }
}
