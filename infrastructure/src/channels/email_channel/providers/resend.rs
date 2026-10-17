use serde::Deserialize;

/// Resend configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ResendConfig {
    pub api_key: String,
    pub from_email: String,
    #[serde(default)]
    pub from_name: Option<String>,
    /// Optional SMTP fallback.
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
