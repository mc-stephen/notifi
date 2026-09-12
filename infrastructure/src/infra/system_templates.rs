//! System email templates: the single reader for `assets/templates/`.
//!
//! Layout per template `<name>` (all files optional except as noted):
//! * `<name>.subject.txt` — one-line subject (required).
//! * `<name>.html` — HTML body (required).
//! * `<name>.txt` — plaintext fallback (optional; derived by stripping
//!   tags from the HTML when absent — good enough for system mail).
//!
//! Variables use `{{name}}` placeholders, substituted verbatim. When a
//! template directory or file is missing, built-in fallbacks for the
//! known system templates keep mail flowing in development.
//!
//! The directory resolves from `MailConfig.templates_dir`, defaulting to
//! [`DEFAULT_TEMPLATES_DIR`]. To move templates in the future, change the
//! default here (or set `NOTIFI_TEMPLATES_DIR`) — nothing else reads the
//! filesystem for mail.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::ports::mailer::{MailError, RenderedTemplate, SystemTemplates};

/// Default system template directory (dev CWD is `infrastructure/`).
pub const DEFAULT_TEMPLATES_DIR: &str = "assets/templates";

pub struct FileSystemTemplates {
    dir: PathBuf,
}

impl FileSystemTemplates {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn from_config(config: &crate::infra::config::MailConfig) -> Self {
        Self::new(
            config
                .templates_dir
                .clone()
                .unwrap_or_else(|| DEFAULT_TEMPLATES_DIR.to_string()),
        )
    }

    fn read(&self, name: &str, ext: &str) -> Option<String> {
        let path: PathBuf = [format!("{name}.{ext}")].iter().collect();
        std::fs::read_to_string(self.dir.join(path)).ok()
    }
}

fn substitute(template: &str, vars: &HashMap<String, String>) -> String {
    let mut out = template.to_string();
    for (key, value) in vars {
        out = out.replace(&format!("{{{{{key}}}}}"), value);
    }
    out
}

fn fallback(name: &str) -> Option<RenderedTemplate> {
    match name {
        // Keep in sync with assets/templates/verify-email.*.
        "verify-email" => Some(RenderedTemplate {
            subject: "Verify your Notifi account".to_string(),
            html: "<p>Hi {{name}},</p><p>Confirm your email address:</p><p><a href=\"{{link}}\">{{link}}</a></p>".to_string(),
            text: "Hi {{name}},\n\nConfirm your email address:\n{{link}}\n".to_string(),
        }),
        // Keep in sync with assets/templates/password-reset.*.
        "password-reset" => Some(RenderedTemplate {
            subject: "Reset your Notifi password".to_string(),
            html: "<p>Hi {{name}},</p><p>Reset your password (valid for one hour):</p><p><a href=\"{{link}}\">{{link}}</a></p><p>If you did not ask for this, ignore this email.</p>".to_string(),
            text: "Hi {{name}},\n\nReset your password (valid for one hour):\n{{link}}\n\nIf you did not ask for this, ignore this email.\n".to_string(),
        }),
        // Keep in sync with assets/templates/password-changed.*.
        "password-changed" => Some(RenderedTemplate {
            subject: "Your Notifi password was changed".to_string(),
            html: "<p>Hi {{name}},</p><p>The password on your Notifi account was just changed. All other sessions were signed out.</p><p><a href=\"{{link}}\">Sign in</a></p><p>If this was not you, reset your password immediately.</p>".to_string(),
            text: "Hi {{name}},\n\nThe password on your Notifi account was just changed. All other sessions were signed out.\n\nSign in:\n{{link}}\n\nIf this was not you, reset your password immediately.\n".to_string(),
        }),
        // Keep in sync with assets/templates/login-notice.*.
        "login-notice" => Some(RenderedTemplate {            subject: "New sign-in to your Notifi account".to_string(),
            html: "<p>Hi {{name}},</p><p>Your Notifi account was just signed in to {{when}}.</p><p><a href=\"{{link}}\">Review my account</a></p><p>If this was you, no action is needed.</p>".to_string(),
            text: "Hi {{name}},\n\nYour Notifi account was just signed in to {{when}}.\n\nReview your account:\n{{link}}\n\nIf this was you, no action is needed.\n".to_string(),
        }),
        // Keep in sync with assets/templates/member-invite.*.
        "member-invite" => Some(RenderedTemplate {
            subject: "You've been invited to {{project}} on Notifi".to_string(),
            html: "<p>Hi {{name}},</p><p>You've been invited to join the {{project}} project on Notifi as {{role}}.</p><p><a href=\"{{link}}\">Review invitation</a></p>".to_string(),
            text: "Hi {{name}},\n\nYou've been invited to join the {{project}} project on Notifi as {{role}}.\n\nReview your invitation:\n{{link}}\n".to_string(),
        }),
        _ => None,
    }
}

impl SystemTemplates for FileSystemTemplates {
    fn render(&self, name: &str, vars: &[(&str, &str)]) -> Result<RenderedTemplate, MailError> {
        let vars: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        let path = Path::new(name);
        if path.components().count() != 1 {
            return Err(MailError::Template(format!(
                "invalid template name: {name}"
            )));
        }

        let subject = match self.read(name, "subject.txt") {
            Some(s) => s,
            None => fallback(name)
                .map(|f| f.subject)
                .ok_or_else(|| MailError::Template(format!("unknown template: {name}")))?,
        };
        let html = match self.read(name, "html") {
            Some(h) => h,
            None => fallback(name)
                .map(|f| f.html)
                .ok_or_else(|| MailError::Template(format!("unknown template: {name}")))?,
        };
        let text = match self.read(name, "txt") {
            Some(t) => t,
            None => fallback(name).map(|f| f.text).unwrap_or_else(|| {
                // Crude tag strip — fine for a plaintext fallback.
                let mut text = String::with_capacity(html.len());
                let mut in_tag = false;
                for c in html.chars() {
                    match c {
                        '<' => in_tag = true,
                        '>' => in_tag = false,
                        _ if !in_tag => text.push(c),
                        _ => {}
                    }
                }
                text
            }),
        };

        Ok(RenderedTemplate {
            subject: substitute(subject.trim(), &vars),
            html: substitute(&html, &vars),
            text: substitute(&text, &vars),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_vars_and_falls_back_without_files() {
        let reader = FileSystemTemplates::new("/nonexistent-dir-xyz");
        let rendered = reader
            .render("verify-email", &[("name", "Ada"), ("link", "https://x/y")])
            .unwrap();
        assert_eq!(rendered.subject, "Verify your Notifi account");
        assert!(rendered.html.contains("https://x/y"));
        assert!(!rendered.html.contains("{{link}}"));
        assert!(rendered.text.contains("Ada"));
    }

    #[test]
    fn login_notice_and_invite_fall_back_without_files() {
        let reader = FileSystemTemplates::new("/nonexistent-dir-xyz");
        let rendered = reader
            .render(
                "login-notice",
                &[
                    ("name", "Ada"),
                    ("when", "June 1, 2026"),
                    ("link", "https://x/profile"),
                ],
            )
            .unwrap();
        assert_eq!(rendered.subject, "New sign-in to your Notifi account");
        assert!(rendered.html.contains("June 1, 2026"));
        assert!(!rendered.html.contains("{{when}}"));

        let rendered = reader
            .render(
                "member-invite",
                &[
                    ("name", "Ada"),
                    ("project", "Acme"),
                    ("role", "developer"),
                    ("link", "https://x/invite/abc"),
                ],
            )
            .unwrap();
        assert_eq!(rendered.subject, "You've been invited to Acme on Notifi");
        assert!(rendered.html.contains("https://x/invite/abc"));
        assert!(!rendered.html.contains("{{"));
        assert!(rendered.text.contains("developer"));
    }

    #[test]
    fn password_changed_falls_back_without_files() {
        let reader = FileSystemTemplates::new("/nonexistent-dir-xyz");
        let rendered = reader
            .render(
                "password-changed",
                &[("name", "Ada"), ("link", "https://x/auth/login")],
            )
            .unwrap();
        assert_eq!(rendered.subject, "Your Notifi password was changed");
        assert!(rendered.html.contains("https://x/auth/login"));
        assert!(!rendered.html.contains("{{link}}"));
    }

    #[test]
    fn unknown_template_errors() {
        let reader = FileSystemTemplates::new("/nonexistent-dir-xyz");
        assert!(matches!(
            reader.render("nope", &[]),
            Err(MailError::Template(_))
        ));
    }

    #[test]
    fn rejects_path_traversal() {
        let reader = FileSystemTemplates::new("/nonexistent-dir-xyz");
        assert!(matches!(
            reader.render("../secret", &[]),
            Err(MailError::Template(_))
        ));
    }
}
