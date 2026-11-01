//! The shared config convention — slugs, keys, and the code-owned default
//! layout (§18 of ARCHITECTURE).
//!
//! * [`project_slug`] — the ONE slug: DB-assigned at creation, unique,
//!   rename-proof; every storage key under `projects/{slug}/…` uses it.
//! * [`ConfigResolver`] — the home of the key convention:
//!   `project_prefix` (the folder convention itself, reused by purge,
//!   export, and tests) plus `channel_key`/`template_key` (storage keys).
//! * [`DefaultProjectLayout`] — the default file set a fresh project
//!   materializes on creation; storage holds no defaults, so reshaping
//!   the layout is purely a code change here.
//!
//! Proven by the unit tests below and the file-driver roundtrip in
//! `server/tests/config_store_file.rs`.

/// The home of the storage-key convention: `projects/{slug}/…`, where
/// `{slug}` is the project's stored slug. Every object-store namespace
/// and test fake addresses the same relative keys.
pub struct ConfigResolver;

impl ConfigResolver {
    /// Storage-key convention root for one project:
    /// `projects/{slug}/` — the single home of the folder convention,
    /// reused by emulation (purge, export) and tests so the prefix is
    /// never hand-built twice.
    pub fn project_prefix(slug: &str) -> String {
        format!("projects/{slug}/")
    }

    /// Storage key for a channel config file:
    /// `projects/{slug}/config/{channel_name}/{file_name}`.
    ///
    /// Relative to whichever [`crate::ports::ConfigStore`] is active (the
    /// file driver prepends the root it is constructed with, the object
    /// store the bucket), so the same key addresses disk, Redis-cached
    /// copies, and R2 buckets alike.
    pub fn channel_key(slug: &str, channel_name: &str, file_name: &str) -> String {
        format!("projects/{slug}/config/{channel_name}/{file_name}")
    }

    /// Storage key for a project template file:
    /// `projects/{slug}/templates/{template_name}/{file_name}`.
    pub fn template_key(slug: &str, template_name: &str, file_name: &str) -> String {
        format!("projects/{slug}/templates/{template_name}/{file_name}")
    }
}

/// The single source of truth for "project name → storage slug".
///
/// Runtime project-folder keys (`projects/{slug}/…`) are keyed by the
/// project's stored slug; the database assigns it from this function at
/// creation and keeps it stable. Storage slugs thus stay unique and
/// rename-proof, unlike a name-derived key. Mirrors the historical
/// PostgreSQL slugifier: lowercase ASCII alphanumerics kept, separators
/// collapsed and trimmed to dashes, `project` as an unsalvageable
/// fallback.
pub fn project_slug(name: &str) -> String {
    let mut slug = String::new();
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if matches!(ch, ' ' | '-' | '_' | '.' | '/') && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let trimmed = slug.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "project".to_string()
    } else {
        trimmed
    }
}

/// A single default file materialized into a fresh project's namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultProjectFile {
    /// Convention key (see [`ConfigResolver::channel_key`] /
    /// [`ConfigResolver::template_key`]).
    pub key: String,
    pub contents: Vec<u8>,
}

/// Default email channel config — SMTP-over-SendGrid placeholder values
/// that the dashboard provider flow replaces tenant-side.
const DEFAULT_EMAIL_CONFIG: &str = r#"{
  "host": "smtp.sendgrid.net",
  "port": 587,
  "username": "apikey",
  "password": "SG.example123",
  "from_address": "noreply@acme.com"
}
"#;

/// The default project layout — owned by code, not by a folder.
///
/// The old reference folder used to carry the structure; it now lives
/// here so re-arranging the shape is a code change every future project
/// inherits (edit this struct, done), and storage holds no ghost copy of
/// it (a reset leaves nothing under `projects/`, projects materialize
/// on creation).
///
/// `config/` gathers each notification channel's config files;
/// `templates/` gathers template files per template name. Contents are
/// transplanted byte-for-byte from the folder this replaced.
pub struct DefaultProjectLayout;

impl DefaultProjectLayout {
    /// Every default file a fresh project gets, in stable order.
    pub fn files(slug: &str) -> Vec<DefaultProjectFile> {
        vec![
            DefaultProjectFile {
                key: ConfigResolver::channel_key(slug, "email", "config.json"),
                contents: DEFAULT_EMAIL_CONFIG.as_bytes().to_vec(),
            },
            DefaultProjectFile {
                key: ConfigResolver::template_key(slug, "welcome", "email.html"),
                contents: DEFAULT_WELCOME_EMAIL_HTML.as_bytes().to_vec(),
            },
            DefaultProjectFile {
                key: ConfigResolver::template_key(slug, "welcome", "sms.txt"),
                contents: DEFAULT_WELCOME_SMS_TEXT.as_bytes().to_vec(),
            },
        ]
    }
}

/// Starter welcome email — the `{{var}}` placeholders and `{{#if}}`
/// blocks ride the shared templating engine at send time.
const DEFAULT_WELCOME_EMAIL_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Welcome to Notifi</title>
</head>
<body style="margin:0;padding:0;background:#f4f4f5;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif">
  <table role="presentation" width="100%" cellpadding="0" cellspacing="0">
    <tr>
      <td align="center" style="padding:40px 16px">
        <table role="presentation" width="480" cellpadding="0" cellspacing="0" style="background:#ffffff;border-radius:12px;overflow:hidden">
          <tr>
            <td style="padding:32px 32px 0">
              <h1 style="margin:0;font-size:24px;color:#18181b">Welcome, {{name}}!</h1>
              <p style="margin:12px 0 0;font-size:15px;line-height:1.6;color:#52525b">{{body}}</p>
            </td>
          </tr>
          {{#if action_url}}
          <tr>
            <td align="center" style="padding:24px 32px">
              <a href="{{action_url}}" style="display:inline-block;padding:12px 28px;background:#18181b;color:#ffffff;border-radius:8px;text-decoration:none;font-size:15px;font-weight:600">Get Started</a>
            </td>
          </tr>
          {{/if}}
          <tr>
            <td style="padding:24px 32px 32px;border-top:1px solid #e4e4e7">
              <p style="margin:0;font-size:13px;color:#a1a1aa">
                Sent from Notifi &mdash; <a href="{{unsubscribe_url}}" style="color:#a1a1aa">Unsubscribe</a>
              </p>
            </td>
          </tr>
        </table>
      </td>
    </tr>
  </table>
</body>
</html>
"##;

/// Starter welcome SMS — compact on purpose; the `{{#if action_url}}`
/// block renders only when an action link exists.
const DEFAULT_WELCOME_SMS_TEXT: &str =
    "Welcome, {{name}}! {{body}} {{#if action_url}}{{action_url}}{{/if}} Reply STOP to opt out.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_key_matches_path_convention() {
        assert_eq!(
            ConfigResolver::channel_key("acme", "email", "config.json"),
            "projects/acme/config/email/config.json"
        );
    }

    #[test]
    fn template_key_matches_path_convention() {
        assert_eq!(
            ConfigResolver::template_key("acme", "welcome", "email.html"),
            "projects/acme/templates/welcome/email.html"
        );
    }

    #[test]
    fn project_prefix_is_the_single_convention_root() {
        assert_eq!(ConfigResolver::project_prefix("acme"), "projects/acme/");
        // The key builders have to stay prefix-consistent with it.
        assert!(
            ConfigResolver::channel_key("acme", "email", "config.json")
                .starts_with(ConfigResolver::project_prefix("acme").as_str())
        );
    }

    #[test]
    fn project_slug_examples() {
        assert_eq!(project_slug("Notifi"), "notifi");
        assert_eq!(project_slug("  A  Corp  "), "a-corp");
        assert_eq!(project_slug("team!! now"), "team-now");
        assert_eq!(project_slug("   "), "project");
    }

    #[test]
    fn layout_files_cover_config_and_templates_per_convention() {
        let files = DefaultProjectLayout::files("acme");
        let keys: Vec<&str> = files.iter().map(|f| f.key.as_str()).collect();
        assert!(
            keys.contains(&ConfigResolver::channel_key("acme", "email", "config.json").as_str())
        );
        assert!(
            keys.contains(&ConfigResolver::template_key("acme", "welcome", "email.html").as_str())
        );
        assert!(
            keys.contains(&ConfigResolver::template_key("acme", "welcome", "sms.txt").as_str())
        );
    }
}
