//! Web push (browser) configuration and controller.
use serde::Deserialize;
use web_push::{
    ContentEncoding, SubscriptionInfo, VapidSignatureBuilder, WebPushClient, WebPushMessageBuilder,
};

/// Web push configuration — the VAPID keypair identifies the server to
/// browsers; `contact_email` rides the VAPID claims.
#[derive(Debug, Deserialize, Clone)]
pub struct WebConfig {
    pub vapid_public_key: String,
    pub vapid_private_key: String,
    pub contact_email: String,
}

/// Web push destination: the browser subscription.
pub struct WebMessage {
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
    pub text: String,
}

/// Web (browser push) device provider controller.
pub struct WebProvider {
    config: WebConfig,
    client: WebPushClient,
}

impl WebProvider {
    pub fn new(config: WebConfig) -> Result<Self, String> {
        let client =
            WebPushClient::new().map_err(|e| format!("Failed to create web push client: {e}"))?;
        Ok(Self { config, client })
    }

    pub async fn send_notification(&self, msg: &WebMessage) -> Result<(), String> {
        let subscription_info = SubscriptionInfo::new(&msg.endpoint, &msg.p256dh, &msg.auth);

        let mut sig_builder = VapidSignatureBuilder::from_pem(
            self.config.vapid_private_key.as_bytes(),
            &subscription_info,
        )
        .map_err(|e| format!("VAPID signature builder failed: {e}"))?;

        sig_builder.add_claim("sub", &*self.config.contact_email);

        let signature = sig_builder
            .build()
            .map_err(|e| format!("Failed to build signature: {e}"))?;

        let mut builder = WebPushMessageBuilder::new(&subscription_info)
            .map_err(|e| format!("Failed to build message: {e}"))?;
        builder.set_payload(ContentEncoding::Aes128Gcm, msg.text.as_bytes());
        builder.set_vapid_signature(signature);

        let message = builder
            .build()
            .map_err(|e| format!("Failed to build message: {e}"))?;

        self.client
            .send(message)
            .await
            .map_err(|e| format!("Web push failed: {e}"))?;

        Ok(())
    }
}

#[cfg(test)]
mod web_tests {
    use super::*;

    fn config() -> WebConfig {
        WebConfig {
            vapid_public_key: "BEl62iUYgUivxIkv69yViEuiBIa".to_string(),
            vapid_private_key: "-----BEGIN PRIVATE KEY-----\nMIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQ\n-----END PRIVATE KEY-----\n".to_string(),
            contact_email: "ops@notifi.dev".to_string(),
        }
    }

    #[test]
    fn config_parses_vapid_triple() {
        let c: WebConfig = serde_json::from_str(
            r#"{"vapid_public_key":"pub","vapid_private_key":"priv","contact_email":"a@b.c"}"#,
        )
        .expect("parses");
        assert_eq!(c.vapid_public_key, "pub");
        assert_eq!(c.contact_email, "a@b.c");
    }

    #[test]
    fn new_builds_a_client() {
        WebProvider::new(config()).expect("client constructs");
    }

    #[tokio::test]
    #[ignore = "live web push — needs NOTIFI_WEB_VAPID_PRIVATE_KEY/_PUBLIC_KEY/_CONTACT and \
                a real browser subscription: NOTIFI_WEB_SUB_ENDPOINT/_P256DH/_AUTH"]
    async fn live_web_push_delivery() {
        let config = WebConfig {
            vapid_public_key: std::env::var("NOTIFI_WEB_VAPID_PUBLIC_KEY")
                .expect("NOTIFI_WEB_VAPID_PUBLIC_KEY not set"),
            vapid_private_key: std::env::var("NOTIFI_WEB_VAPID_PRIVATE_KEY")
                .expect("NOTIFI_WEB_VAPID_PRIVATE_KEY not set"),
            contact_email: std::env::var("NOTIFI_WEB_VAPID_CONTACT")
                .expect("NOTIFI_WEB_VAPID_CONTACT not set"),
        };
        let web = WebProvider::new(config).expect("client constructs");
        web.send_notification(&WebMessage {
            endpoint: std::env::var("NOTIFI_WEB_SUB_ENDPOINT")
                .expect("NOTIFI_WEB_SUB_ENDPOINT not set"),
            p256dh: std::env::var("NOTIFI_WEB_SUB_P256DH").expect("NOTIFI_WEB_SUB_P256DH not set"),
            auth: std::env::var("NOTIFI_WEB_SUB_AUTH").expect("NOTIFI_WEB_SUB_AUTH not set"),
            text: format!(
                "Notifi live-check {}",
                chrono::Utc::now().timestamp_millis()
            ),
        })
        .await
        .expect("real web push endpoint accepted the message");
    }
}
