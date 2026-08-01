mod config;

pub use config::WebConfig;

use web_push::{
    ContentEncoding, SubscriptionInfo, VapidSignatureBuilder, WebPushClient, WebPushMessageBuilder,
};

pub struct WebMessage {
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
    pub text: String,
}

pub struct WebProvider {
    config: WebConfig,
    client: WebPushClient,
}

impl WebProvider {
    pub fn new(config: WebConfig) -> Self {
        Self {
            config,
            client: WebPushClient::new(),
        }
    }

    pub async fn send_notification(&self, msg: &WebMessage) -> Result<(), String> {
        let subscription_info = SubscriptionInfo::new(&msg.endpoint, &msg.p256dh, &msg.auth);

        let mut sig_builder = VapidSignatureBuilder::from_vapid_key_pair(
            &web_push::VapidKey::from_pem(
                &self.config.vapid_private_key.as_bytes(),
                &self.config.vapid_public_key.as_bytes(),
            )
            .map_err(|e| format!("Invalid VAPID keys: {e}"))?,
        )
        .map_err(|e| format!("VAPID signature builder failed: {e}"))?;

        sig_builder.add_claim("sub", &self.config.contact_email);

        let signature = sig_builder
            .build()
            .map_err(|e| format!("Failed to build signature: {e}"))?;

        let mut builder = WebPushMessageBuilder::new(&subscription_info);
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
