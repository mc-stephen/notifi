use serde::Deserialize;

/// Twilio SendGrid configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct SendGridConfig {
    pub api_key: String,
    pub from_email: String,
    #[serde(default)]
    pub from_name: Option<String>,
    /// Optional SMTP fallback — SendGrid supports SMTP on port 587.
    #[serde(default)]
    pub smtp: Option<SmtpFallback>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SmtpFallback {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    #[serde(default = "default_tls")]
    pub tls: bool,
}

fn default_tls() -> bool {
    true
}
