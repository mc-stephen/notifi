use serde::Deserialize;

/// AWS Simple Email Service configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct AwsSesConfig {
    pub access_key: String,
    pub secret_key: String,
    pub region: String,
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
