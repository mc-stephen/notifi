//! System mail contracts: template rendering + SMTP delivery.
//!
//! Framework-free by rule: no axum/sqlx/reqwest types may appear here.

use std::future::Future;
use std::pin::Pin;

/// Boxed future returned by every port method.
pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A rendered system email template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedTemplate {
    pub subject: String,
    pub html: String,
    pub text: String,
}

/// Failure modes for system mail. Delivery failures are non-fatal to
/// callers (flows keep their dev behavior); they surface in logs/tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MailError {
    /// No mailer is configured.
    Disabled,
    /// Unknown template name.
    Template(String),
    /// The SMTP transaction failed.
    Transport(String),
}

impl std::fmt::Display for MailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disabled => write!(f, "system mail is not configured"),
            Self::Template(m) => write!(f, "template error: {m}"),
            Self::Transport(m) => write!(f, "SMTP error: {m}"),
        }
    }
}

impl std::error::Error for MailError {}

/// Who a system email is sent as. Code-owned identities — never free-form
/// strings at call sites — so automated mail can never accidentally wear a
/// human address (and vice versa).
///
/// * `NoReply` — all automated system mail today (verification, resets,
///   login notices, team invites, admin resets).
/// * `Support` — human-sent mail from the admin dashboard (account
///   managers emailing users). Reserved; no sender uses it yet.
/// * `Custom` — a future admin-configured sender (stored + managed from
///   the admin dashboard). Validated as a bare email when resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemSender {
    NoReply,
    Support,
    Custom { email: String, name: Option<String> },
}

impl SystemSender {
    /// Resolves to `(bare email, display name)` for `domain`
    /// (e.g. `NOTIFI_MAIL_DOMAIN`). Rejects malformed custom addresses.
    pub fn resolve(&self, domain: &str) -> Result<(String, Option<String>), MailError> {
        let domain = domain.trim().trim_start_matches('@');
        match self {
            Self::NoReply => Ok((format!("no-reply@{domain}"), Some("Notifi".to_string()))),
            Self::Support => Ok((
                format!("support@{domain}"),
                Some("Notifi Support".to_string()),
            )),
            Self::Custom { email, name } => {
                let email = email.trim();
                if !is_bare_email(email) {
                    return Err(MailError::Template(format!(
                        "invalid custom sender address: {email}"
                    )));
                }
                Ok((email.to_string(), name.clone()))
            }
        }
    }
}

/// Minimal bare-email check (`local@domain`): enough to reject display
/// names and blanks before lettre parses the address strictly.
fn is_bare_email(value: &str) -> bool {
    if value.contains(['<', '>', ' ', ',']) {
        return false;
    }
    let mut parts = value.split('@');
    matches!((parts.next(), parts.next(), parts.next()), (Some(l), Some(d), None) if !l.is_empty() && !d.is_empty() && d.contains('.'))
}

/// Renders system email templates by name with `{{var}}` substitution.
pub trait SystemTemplates: Send + Sync {
    fn render(&self, name: &str, vars: &[(&str, &str)]) -> Result<RenderedTemplate, MailError>;
}

/// Sends one system email. Returns the provider message id on success.
pub trait SmtpMailer: Send + Sync {
    fn send(
        &self,
        from: SystemSender,
        to: &str,
        subject: &str,
        text: &str,
        html: Option<&str>,
    ) -> BoxFut<'_, Result<String, MailError>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_senders_resolve_against_the_domain() {
        let (email, name) = SystemSender::NoReply.resolve("notifi.dev").unwrap();
        assert_eq!(email, "no-reply@notifi.dev");
        assert_eq!(name.as_deref(), Some("Notifi"));
        let (email, name) = SystemSender::Support.resolve("notifi.dev").unwrap();
        assert_eq!(email, "support@notifi.dev");
        assert_eq!(name.as_deref(), Some("Notifi Support"));
    }

    #[test]
    fn custom_sender_must_be_a_bare_email() {
        let ok = SystemSender::Custom {
            email: "ada@example.com".to_string(),
            name: Some("Ada".to_string()),
        };
        assert_eq!(
            ok.resolve("notifi.dev").unwrap(),
            ("ada@example.com".to_string(), Some("Ada".to_string()))
        );
        for bad in ["", "not-an-email", "No Reply <x@y.zz>", "a@b c.zz"] {
            let sender = SystemSender::Custom {
                email: bad.to_string(),
                name: None,
            };
            assert!(
                matches!(sender.resolve("notifi.dev"), Err(MailError::Template(_))),
                "expected {bad:?} to be rejected"
            );
        }
    }
}
