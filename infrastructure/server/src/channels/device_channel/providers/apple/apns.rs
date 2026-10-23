use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// iOS (APNs) configuration.
///
/// `p8_key_path` points at the Apple signing key; the JWT `iss` claim is
/// the team id and the `apns-topic` header the bundle id. `sandbox`
/// selects Apple's sandbox gateway for dev builds; it defaults to
/// production, since an omitted flag silently-sandboxing real sends
/// would be the worse surprise.
#[derive(Debug, Clone, Deserialize)]
pub struct IosConfig {
    pub key_id: String,
    pub team_id: String,
    pub bundle_id: String,
    pub p8_key_path: PathBuf,
    #[serde(default)]
    pub sandbox: bool,
}

/// macOS (APNs) configuration — the same APNs signing flow as iOS.
#[derive(Debug, Clone, Deserialize)]
pub struct MacosConfig {
    pub key_id: String,
    pub team_id: String,
    pub bundle_id: String,
    pub p8_key_path: PathBuf,
    #[serde(default)]
    pub sandbox: bool,
}

/// Apple device destination: the APNs device token.
pub struct ApnsMessage {
    pub device_token: String,
    pub title: String,
    pub body: String,
}

#[derive(Serialize)]
struct Claims {
    iss: String,
    iat: u64,
}

/// Shared APNs controller for both iOS and macOS destinations. The two
/// flows were identical down to the byte; the flavor lives in the config,
/// not in this code.
pub struct ApnsProvider {
    team_id: String,
    bundle_id: String,
    p8_key_path: PathBuf,
    sandbox: bool,
    client: Client,
    /// APNs base (scheme + host) override for tests; production never
    /// sets it.
    #[cfg(test)]
    apns_base: Option<String>,
}

impl ApnsProvider {
    /// Builds the shared controller from either OS flavor's config.
    pub fn new(config: IosConfig) -> Result<Self, String> {
        Ok(Self {
            team_id: config.team_id,
            bundle_id: config.bundle_id,
            p8_key_path: config.p8_key_path,
            sandbox: config.sandbox,
            client: Client::builder()
                .build()
                .map_err(|e| format!("Failed to build APNs HTTP client: {e}"))?,
            #[cfg(test)]
            apns_base: None,
        })
    }

    pub fn new_macos(config: MacosConfig) -> Result<Self, String> {
        ApnsProvider::new(IosConfig {
            key_id: config.key_id,
            team_id: config.team_id,
            bundle_id: config.bundle_id,
            p8_key_path: config.p8_key_path,
            sandbox: config.sandbox,
        })
    }

    #[cfg(test)]
    pub(crate) fn new_with_base(config: IosConfig, base: String) -> Self {
        Self {
            #[cfg(test)]
            apns_base: Some(base),
            ..Self::new(config).expect("test config builds")
        }
    }

    /// The production default honors the sandbox flag: dev builds sign
    /// against Apple's sandbox gateway.
    fn default_base(&self) -> &'static str {
        if self.sandbox {
            "https://api.sandbox.push.apple.com"
        } else {
            "https://api.push.apple.com"
        }
    }

    fn generate_jwt(&self) -> Result<String, String> {
        let p8_key = fs::read_to_string(&self.p8_key_path)
            .map_err(|e| format!("Failed to read p8 key: {e}"))?;

        let header = Header::new(Algorithm::ES256);
        let claims = Claims {
            iss: self.team_id.clone(),
            iat: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let encoding_key = EncodingKey::from_ec_pem(p8_key.as_bytes())
            .map_err(|e| format!("Failed to create encoding key: {e}"))?;

        encode(&header, &claims, &encoding_key).map_err(|e| format!("Failed to encode JWT: {e}"))
    }

    /// The APNs POST itself — split from JWT signing so tests drive the
    /// wire shape with a dummy token, no Apple key required.
    pub(crate) async fn post_apns(&self, jwt: &str, msg: &ApnsMessage) -> Result<(), String> {
        let url = {
            #[cfg(test)]
            {
                match &self.apns_base {
                    Some(base) => format!("{base}/3/device/{}", msg.device_token),
                    None => format!("{}/3/device/{}", self.default_base(), msg.device_token),
                }
            }
            #[cfg(not(test))]
            {
                format!("{}/3/device/{}", self.default_base(), msg.device_token)
            }
        };
        let payload = json!({
            "aps": {
                "alert": { "title": msg.title, "body": msg.body }
            }
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {jwt}"))
            .header("apns-topic", &self.bundle_id)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("APNs request failed: {e}"))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let err = response.text().await.unwrap_or_default();
            Err(format!("APNs error: {err}"))
        }
    }

    pub async fn send_notification(&self, msg: &ApnsMessage) -> Result<(), String> {
        let jwt = self.generate_jwt()?;
        self.post_apns(&jwt, msg).await
    }
}

#[cfg(test)]
mod apns_tests {
    use super::*;

    fn ios_config() -> IosConfig {
        IosConfig {
            key_id: "KEYID1".to_string(),
            team_id: "TEAMID9".to_string(),
            bundle_id: "app.notifi.test".to_string(),
            p8_key_path: PathBuf::from("/nonexistent/authkey.p8"),
            sandbox: false,
        }
    }

    fn msg() -> ApnsMessage {
        ApnsMessage {
            device_token: "tok-123".to_string(),
            title: "Hi".to_string(),
            body: "Body".to_string(),
        }
    }

    #[test]
    fn macos_config_maps_onto_the_shared_controller() {
        let provider = ApnsProvider::new_macos(MacosConfig {
            key_id: "K".to_string(),
            team_id: "T".to_string(),
            bundle_id: "b.1".to_string(),
            p8_key_path: PathBuf::from("/x/key.p8"),
            sandbox: false,
        })
        .expect("builds");
        assert_eq!(provider.team_id, "T");
        assert_eq!(provider.bundle_id, "b.1");
    }

    #[test]
    fn configs_deserialize_both_flavors() {
        let ios: IosConfig = serde_json::from_str(
            r#"{"key_id":"k","team_id":"t","bundle_id":"b","p8_key_path":"/k.p8"}"#,
        )
        .expect("ios parses");
        assert_eq!(ios.team_id, "t");
        assert!(!ios.sandbox, "omitted flag defaults to production");
        let _: MacosConfig = serde_json::from_str(
            r#"{"key_id":"k","team_id":"t","bundle_id":"b","p8_key_path":"/k.p8"}"#,
        )
        .expect("macos parses");
    }

    #[test]
    fn sandbox_flag_picks_the_gateway() {
        let prod = ApnsProvider::new(ios_config()).expect("builds");
        assert_eq!(prod.default_base(), "https://api.push.apple.com");
        let sandbox = ApnsProvider::new(IosConfig {
            sandbox: true,
            ..ios_config()
        })
        .expect("builds");
        assert_eq!(sandbox.default_base(), "https://api.sandbox.push.apple.com");
    }

    #[tokio::test]
    async fn posts_apns_payload_with_bearer_and_topic() {
        let stub = crate::testing::http_stub::start(200, "OK", "").await;
        ApnsProvider::new_with_base(ios_config(), format!("http://127.0.0.1:{}", stub.port))
            .post_apns("dummy-jwt", &msg())
            .await
            .expect("stub 200 delivers");
        let raw = stub.request().await.expect("request captured");
        assert!(
            raw.contains(&format!("POST /3/device/{}", msg().device_token)),
            "device token path: {raw}"
        );
        assert!(
            raw.to_ascii_lowercase()
                .contains("authorization: bearer dummy-jwt"),
            "bearer header: {raw}"
        );
        assert!(
            raw.contains("apns-topic: app.notifi.test"),
            "topic header: {raw}"
        );
        assert!(raw.contains(r#""alert""#), "payload: {raw}");
    }

    #[tokio::test]
    async fn surfaces_apns_error_body() {
        let stub = crate::testing::http_stub::start(400, "Bad Request", "BadDeviceToken").await;
        let err =
            ApnsProvider::new_with_base(ios_config(), format!("http://127.0.0.1:{}", stub.port))
                .post_apns("dummy-jwt", &msg())
                .await
                .expect_err("400 is a failed APNs send");
        assert!(err.starts_with("APNs error:"), "prefix: {err}");
        assert!(err.contains("BadDeviceToken"), "error body surfaced: {err}");
    }

    #[tokio::test]
    #[ignore = "live APNs delivery — needs a real .p8 key, NOTIFI_APNS_TEAM_ID, \
                NOTIFI_APNS_BUNDLE_ID, and a true device token to reach a registered device"]
    async fn live_apns_delivery() {
        let config = IosConfig {
            key_id: std::env::var("NOTIFI_APNS_KEY_ID").expect("NOTIFI_APNS_KEY_ID not set"),
            team_id: std::env::var("NOTIFI_APNS_TEAM_ID").expect("NOTIFI_APNS_TEAM_ID not set"),
            bundle_id: std::env::var("NOTIFI_APNS_BUNDLE_ID")
                .expect("NOTIFI_APNS_BUNDLE_ID not set"),
            p8_key_path: PathBuf::from(
                std::env::var("NOTIFI_APNS_P8_PATH").expect("NOTIFI_APNS_P8_PATH not set"),
            ),
            sandbox: std::env::var("NOTIFI_APNS_SANDBOX")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
        };
        let device_token =
            std::env::var("NOTIFI_APNS_DEVICE_TOKEN").expect("NOTIFI_APNS_DEVICE_TOKEN not set");
        ApnsProvider::new(config)
            .expect("builds")
            .send_notification(&ApnsMessage {
                device_token,
                title: "Notifi live-check".to_string(),
                body: "Direct APNs delivery check".to_string(),
            })
            .await
            .expect("real APNs API accepted the push");
    }
}
