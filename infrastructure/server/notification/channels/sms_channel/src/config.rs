use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(tag = "provider")]
pub enum SmsConfig {
    #[serde(rename = "twilio")]
    Twilio { 
        account_sid: String, 
        auth_token: String, 
        from_number: String 
    },
    #[serde(rename = "local_nigeria")]
    LocalNigeria { 
        api_key: String, 
        base_url: String, 
        sender_id: String 
    },
}

impl SmsConfig {
    pub fn load(brand: &str) -> Result<Self, String> {
        let path: PathBuf = ["configs", brand, "sms", "config.json"].iter().collect();
        let bytes = fs::read(&path).map_err(|e| format!("Failed to read {path:?}: {e}"))?;
        serde_json::from_slice::<Self>(&bytes).map_err(|e| format!("Failed to parse config: {e}"))
    }
}
