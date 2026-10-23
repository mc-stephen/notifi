use serde::de::DeserializeOwned;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;
/// Errors that can occur when loading a tenant channel configuration.
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("config directory not found for project '{project}' channel '{channel}' at {path}")]
    DirectoryNotFound {
        project: String,
        channel: String,
        path: PathBuf,
    },
    #[error("I/O error reading config for project '{project}' channel '{channel}': {source}")]
    Io {
        project: String,
        channel: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config for project '{project}' channel '{channel}': {message}")]
    Parse {
        project: String,
        channel: String,
        message: String,
    },
}

/// Resolves the filesystem path for a tenant's channel configuration.
///
/// Canonical layout under the config root (`NOTIFI_CONFIG_ROOT`, defaults to
/// `configs`):
///
/// ```text
/// {root}/projects/{slug}/config/{channel_name}/...      channel configs
/// {root}/projects/{slug}/templates/{template_name}/...   project-scoped templates
/// ```
///
/// The root is the assets root: `projects/` sits directly under it; the
/// `{slug}` segment is the project's stored slug.
pub struct ConfigResolver;

impl ConfigResolver {
    /// Returns the root directory under which all project dirs live.
    /// Override with the `NOTIFI_CONFIG_ROOT` env var (defaults to `configs`).
    ///
    /// `infra` config-store drivers resolve their base from here too, so
    /// the file driver and the kernel paths can never drift apart.
    pub fn root() -> PathBuf {
        std::env::var("NOTIFI_CONFIG_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("configs"))
    }

    /// Storage-key convention root for one project:
    /// `projects/{slug}/` — the single home of the folder convention,
    /// reused by emulation (purge, export) and tests so the prefix is
    /// never hand-built twice.
    pub fn project_prefix(slug: &str) -> String {
        format!("projects/{slug}/")
    }

    /// Builds the channel config path: `{root}/projects/{slug}/config/{channel_name}/`.
    pub fn channel_dir(slug: &str, channel_name: &str) -> PathBuf {
        Self::root()
            .join("projects")
            .join(slug)
            .join("config")
            .join(channel_name)
    }

    /// Builds the template path: `{root}/projects/{slug}/templates/{template_name}/`.
    pub fn template_dir(slug: &str, template_name: &str) -> PathBuf {
        Self::root()
            .join("projects")
            .join(slug)
            .join("templates")
            .join(template_name)
    }

    /// Storage key for a channel config file:
    /// `projects/{slug}/config/{channel_name}/{file_name}`.
    ///
    /// Relative to whichever [`crate::ports::ConfigStore`] is active (the
    /// file driver prepends the same root these path helpers use), so the
    /// same key addresses disk, Redis-cached copies, and R2 buckets alike.
    pub fn channel_key(slug: &str, channel_name: &str, file_name: &str) -> String {
        format!("projects/{slug}/config/{channel_name}/{file_name}")
    }

    /// Storage key for a project template file:
    /// `projects/{slug}/templates/{template_name}/{file_name}`.
    pub fn template_key(slug: &str, template_name: &str, file_name: &str) -> String {
        format!("projects/{slug}/templates/{template_name}/{file_name}")
    }

    /// Reads and parses a JSON config file for a tenant channel.
    ///
    /// Path: `{root}/projects/{slug}/config/{channel_name}/{file_name}`.
    ///
    /// Returns [`ConfigError::DirectoryNotFound`] when the directory does not
    /// exist, which lets channels distinguish "no config installed for this
    /// tenant" from a broken config.
    pub fn load_json<T: DeserializeOwned>(
        slug: &str,
        channel_name: &str,
        file_name: &str,
    ) -> Result<T, ConfigError> {
        let dir = Self::channel_dir(slug, channel_name);
        if !dir.is_dir() {
            return Err(ConfigError::DirectoryNotFound {
                project: slug.to_string(),
                channel: channel_name.to_string(),
                path: dir,
            });
        }

        let path = dir.join(file_name);
        let bytes = fs::read(&path).map_err(|source| ConfigError::Io {
            project: slug.to_string(),
            channel: channel_name.to_string(),
            source,
        })?;

        serde_json::from_slice(&bytes).map_err(|e| ConfigError::Parse {
            project: slug.to_string(),
            channel: channel_name.to_string(),
            message: e.to_string(),
        })
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

    /// Serializes tests that mutate process env vars.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn set_env(key: &str, value: &str) {
        // SAFETY: process-local mutation, guarded by ENV_LOCK so tests never race.
        unsafe { std::env::set_var(key, value) };
    }

    fn clear_env(key: &str) {
        // SAFETY: process-local mutation, guarded by ENV_LOCK so tests never race.
        unsafe { std::env::remove_var(key) };
    }

    #[derive(serde::Deserialize, PartialEq, Debug)]
    struct StubConfig {
        port: u16,
        host: String,
    }

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

    #[test]
    fn resolves_channel_dir_from_env_root() {
        let _guard = ENV_LOCK.lock().unwrap();
        set_env("NOTIFI_CONFIG_ROOT", "/tmp/notifi-configs");
        let dir = ConfigResolver::channel_dir("acme", "email");
        assert_eq!(
            dir,
            PathBuf::from("/tmp/notifi-configs/projects/acme/config/email")
        );
        clear_env("NOTIFI_CONFIG_ROOT");
    }

    #[test]
    fn resolves_template_dir_from_env_root() {
        let _guard = ENV_LOCK.lock().unwrap();
        set_env("NOTIFI_CONFIG_ROOT", "/tmp/notifi-configs");
        let dir = ConfigResolver::template_dir("acme", "welcome");
        assert_eq!(
            dir,
            PathBuf::from("/tmp/notifi-configs/projects/acme/templates/welcome")
        );
        clear_env("NOTIFI_CONFIG_ROOT");
    }

    #[test]
    fn load_json_reports_missing_directory() {
        let _guard = ENV_LOCK.lock().unwrap();
        set_env("NOTIFI_CONFIG_ROOT", "/nonexistent/notifi");
        let err = ConfigResolver::load_json::<StubConfig>("acme", "email", "config.json")
            .expect_err("should fail");
        assert!(matches!(err, ConfigError::DirectoryNotFound { .. }));
        clear_env("NOTIFI_CONFIG_ROOT");
    }

    #[test]
    fn load_json_parses_file() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = std::env::temp_dir().join("notifi-test-root");
        set_env("NOTIFI_CONFIG_ROOT", root.to_str().unwrap());
        let dir = root
            .join("projects")
            .join("acme")
            .join("config")
            .join("email");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("config.json"),
            r#"{"port": 587, "host": "smtp.example.com"}"#,
        )
        .unwrap();

        let cfg = ConfigResolver::load_json::<StubConfig>("acme", "email", "config.json").unwrap();
        assert_eq!(cfg.host, "smtp.example.com");
        assert_eq!(cfg.port, 587);

        std::fs::remove_dir_all(&root).unwrap();
        clear_env("NOTIFI_CONFIG_ROOT");
    }
}
