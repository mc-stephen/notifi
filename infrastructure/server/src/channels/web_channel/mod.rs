//! TODO(M3 — web_channel): THIS CRATE IS CURRENTLY BROKEN AND EXCLUDED FROM
//! ALL WORKSPACE COMMANDS (`cargo test/clippy/check --exclude web_channel`).
//!
//! Why: the `web-push` crate's API drifted from what this channel uses
//! (subscription/VAPID builder signatures no longer match). The workspace
//! still lists it as a member so it is not forgotten, but every command must
//! exclude it or the build fails.
//!
//! Fix checklist for whoever picks this up (M3 delivery milestone):
//!   1. Verify against the current web-push API surface (`cargo check`).
//!   2. Implement the `DeliveryProvider` port (see `src/ports/delivery.rs`).

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
