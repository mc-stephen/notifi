//! System email templates: bodies live in the config store
//! (`templates/{name}.html` — the R2 bucket in deployment), seeded
//! write-if-missing at boot from the built-ins below (the same
//! code-owned-defaults doctrine as the project starter layout).
//!
//! The warm layer is Redis forever-keys (`cfg:templates/{name}.html`,
//! no expiry) when Redis is configured: renders answer from Redis, a
//! cold name does one store fetch and is SET for good, and a warm pass
//! (boot, or `POST /admin/templates/warm`) re-reads the folder and
//! overwrites — that's how an edit in the bucket lands, on restart or
//! on demand, visible to every instance sharing the Redis. Without
//! Redis the same cache lives in-process for the lifetime of the
//! server instead.
//!
//! Subjects are code — one template, one purpose (the old
//! `.subject.txt` files are gone). Plaintext is derived by stripping
//! tags from the HTML. `{{name}}` variables ride the shared kernel
//! engine, substituted verbatim.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::infra::config_cache::CacheBackend;
use crate::ports::config_store::ConfigStore;
use crate::ports::mailer::{BoxFut, MailError, RenderedTemplate, SystemTemplates};

/// Store folder holding the system template bodies (top level, next to
/// `projects/` and `backup/`).
const TEMPLATES_FOLDER: &str = "templates";

/// Byte-for-byte transplant of the original `assets/templates/verify-email.html`.
const VERIFY_EMAIL_HTML: &str = r##"
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta name="color-scheme" content="light">
  <meta name="supported-color-schemes" content="light">
  <title>Verify your Notifi account</title>
</head>
<body style="margin:0;padding:0;background-color:#F5F5F5;">
  <div style="display:none;max-height:0;overflow:hidden;opacity:0;">
    Confirm your email address to start sending notifications with Notifi.
  </div>
  <table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="background-color:#F5F5F5;">
    <tr>
      <td align="center" style="padding:40px 16px;">
        <table role="presentation" width="600" cellpadding="0" cellspacing="0" style="max-width:600px;background-color:#ffffff;border-radius:12px;overflow:hidden;border:1px solid #E4E4E7;">
          <tr>
            <td style="padding:32px 40px 0;">
              <p style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:20px;font-weight:700;letter-spacing:-0.02em;color:#5B3CD9;">Notifi</p>
            </td>
          </tr>
          <tr>
            <td style="padding:24px 40px 0;">
              <h1 style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:24px;font-weight:700;letter-spacing:-0.01em;color:#0A0A0A;">Verify your email address</h1>
              <p style="margin:12px 0 0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:15px;line-height:1.6;color:#52525B;">Hi {{name}},</p>
              <p style="margin:12px 0 0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:15px;line-height:1.6;color:#52525B;">Thanks for signing up for Notifi. Confirm your email address below to activate your account and start sending notifications.</p>
            </td>
          </tr>
          <tr>
            <td align="center" style="padding:28px 40px 8px;">
              <a href="{{link}}" style="display:inline-block;padding:13px 32px;background-color:#5B3CD9;color:#FAFAFA;border-radius:8px;text-decoration:none;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:15px;font-weight:600;">Verify email address</a>
            </td>
          </tr>
          <tr>
            <td style="padding:16px 40px 0;">
              <p style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:13px;line-height:1.6;color:#71717A;">Button not working? Paste this link into your browser:</p>
              <p style="margin:8px 0 0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:13px;line-height:1.6;word-break:break-all;"><a href="{{link}}" style="color:#5B3CD9;">{{link}}</a></p>
            </td>
          </tr>
          <tr>
            <td style="padding:28px 40px 32px;border-top:1px solid #E4E4E7;">
              <p style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:12px;line-height:1.6;color:#A1A1AA;">If you did not create a Notifi account, you can safely ignore this email. This is an automated message, please do not reply.</p>
            </td>
          </tr>
        </table>
        <table role="presentation" width="600" cellpadding="0" cellspacing="0" style="max-width:600px;">
          <tr>
            <td align="center" style="padding:20px 16px 0;">
              <p style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:12px;color:#A1A1AA;">Notifi &mdash; notifications, delivered.</p>
            </td>
          </tr>
        </table>
      </td>
    </tr>
  </table>
</body>
</html>
"##;

/// Byte-for-byte transplant of the original `assets/templates/password-reset.html`.
const PASSWORD_RESET_HTML: &str = r##"
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta name="color-scheme" content="light">
  <meta name="supported-color-schemes" content="light">
  <title>Reset your Notifi password</title>
</head>
<body style="margin:0;padding:0;background-color:#F5F5F5;">
  <div style="display:none;max-height:0;overflow:hidden;opacity:0;">
    Reset your Notifi password. This link expires in one hour.
  </div>
  <table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="background-color:#F5F5F5;">
    <tr>
      <td align="center" style="padding:40px 16px;">
        <table role="presentation" width="600" cellpadding="0" cellspacing="0" style="max-width:600px;background-color:#ffffff;border-radius:12px;overflow:hidden;border:1px solid #E4E4E7;">
          <tr>
            <td style="padding:32px 40px 0;">
              <p style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:20px;font-weight:700;letter-spacing:-0.02em;color:#5B3CD9;">Notifi</p>
            </td>
          </tr>
          <tr>
            <td style="padding:24px 40px 0;">
              <h1 style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:24px;font-weight:700;letter-spacing:-0.01em;color:#0A0A0A;">Reset your password</h1>
              <p style="margin:12px 0 0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:15px;line-height:1.6;color:#52525B;">Hi {{name}},</p>
              <p style="margin:12px 0 0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:15px;line-height:1.6;color:#52525B;">We received a request to reset the password on your Notifi account. Choose a new password below. <strong>This link expires in one hour.</strong></p>
            </td>
          </tr>
          <tr>
            <td align="center" style="padding:28px 40px 8px;">
              <a href="{{link}}" style="display:inline-block;padding:13px 32px;background-color:#5B3CD9;color:#FAFAFA;border-radius:8px;text-decoration:none;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:15px;font-weight:600;">Reset password</a>
            </td>
          </tr>
          <tr>
            <td style="padding:16px 40px 0;">
              <p style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:13px;line-height:1.6;color:#71717A;">Button not working? Paste this link into your browser:</p>
              <p style="margin:8px 0 0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:13px;line-height:1.6;word-break:break-all;"><a href="{{link}}" style="color:#5B3CD9;">{{link}}</a></p>
            </td>
          </tr>
          <tr>
            <td style="padding:28px 40px 32px;border-top:1px solid #E4E4E7;">
              <p style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:12px;line-height:1.6;color:#A1A1AA;">If you did not request a password reset, you can safely ignore this email — your password will stay the same. This is an automated message, please do not reply.</p>
            </td>
          </tr>
        </table>
        <table role="presentation" width="600" cellpadding="0" cellspacing="0" style="max-width:600px;">
          <tr>
            <td align="center" style="padding:20px 16px 0;">
              <p style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:12px;color:#A1A1AA;">Notifi &mdash; notifications, delivered.</p>
            </td>
          </tr>
        </table>
      </td>
    </tr>
  </table>
</body>
</html>
"##;

/// Byte-for-byte transplant of the original `assets/templates/password-changed.html`.
const PASSWORD_CHANGED_HTML: &str = r##"
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta name="color-scheme" content="light">
  <meta name="supported-color-schemes" content="light">
  <title>Your Notifi password was changed</title>
</head>
<body style="margin:0;padding:0;background-color:#F5F5F5;">
  <div style="display:none;max-height:0;overflow:hidden;opacity:0;">
    The password on your Notifi account was just changed.
  </div>
  <table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="background-color:#F5F5F5;">
    <tr>
      <td align="center" style="padding:40px 16px;">
        <table role="presentation" width="600" cellpadding="0" cellspacing="0" style="max-width:600px;background-color:#ffffff;border-radius:12px;overflow:hidden;border:1px solid #E4E4E7;">
          <tr>
            <td style="padding:32px 40px 0;">
              <p style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:20px;font-weight:700;letter-spacing:-0.02em;color:#5B3CD9;">Notifi</p>
            </td>
          </tr>
          <tr>
            <td style="padding:24px 40px 0;">
              <h1 style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:24px;font-weight:700;letter-spacing:-0.01em;color:#0A0A0A;">Password changed</h1>
              <p style="margin:12px 0 0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:15px;line-height:1.6;color:#52525B;">Hi {{name}},</p>
              <p style="margin:12px 0 0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:15px;line-height:1.6;color:#52525B;">The password on your Notifi account was just changed. All other sessions were signed out as a precaution.</p>
            </td>
          </tr>
          <tr>
            <td align="center" style="padding:28px 40px 8px;">
              <a href="{{link}}" style="display:inline-block;padding:13px 32px;background-color:#5B3CD9;color:#FAFAFA;border-radius:8px;text-decoration:none;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:15px;font-weight:600;">Sign in</a>
            </td>
          </tr>
          <tr>
            <td style="padding:28px 40px 32px;border-top:1px solid #E4E4E7;">
              <p style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:12px;line-height:1.6;color:#A1A1AA;">If this was you, no action is needed. If you did not change your password, your account may be compromised — reset your password immediately and contact support@notifi.dev. This is an automated message, please do not reply.</p>
            </td>
          </tr>
        </table>
        <table role="presentation" width="600" cellpadding="0" cellspacing="0" style="max-width:600px;">
          <tr>
            <td align="center" style="padding:20px 16px 0;">
              <p style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:12px;color:#A1A1AA;">Notifi &mdash; notifications, delivered.</p>
            </td>
          </tr>
        </table>
      </td>
    </tr>
  </table>
</body>
</html>
"##;

/// Byte-for-byte transplant of the original `assets/templates/login-notice.html`.
const LOGIN_NOTICE_HTML: &str = r##"
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta name="color-scheme" content="light">
  <meta name="supported-color-schemes" content="light">
  <title>New sign-in to your Notifi account</title>
</head>
<body style="margin:0;padding:0;background-color:#F5F5F5;">
  <div style="display:none;max-height:0;overflow:hidden;opacity:0;">
    A new sign-in to your Notifi account was just detected.
  </div>
  <table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="background-color:#F5F5F5;">
    <tr>
      <td align="center" style="padding:40px 16px;">
        <table role="presentation" width="600" cellpadding="0" cellspacing="0" style="max-width:600px;background-color:#ffffff;border-radius:12px;overflow:hidden;border:1px solid #E4E4E7;">
          <tr>
            <td style="padding:32px 40px 0;">
              <p style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:20px;font-weight:700;letter-spacing:-0.02em;color:#5B3CD9;">Notifi</p>
            </td>
          </tr>
          <tr>
            <td style="padding:24px 40px 0;">
              <h1 style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:24px;font-weight:700;letter-spacing:-0.01em;color:#0A0A0A;">New sign-in detected</h1>
              <p style="margin:12px 0 0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:15px;line-height:1.6;color:#52525B;">Hi {{name}},</p>
              <p style="margin:12px 0 0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:15px;line-height:1.6;color:#52525B;">Your Notifi account was just signed in to {{when}}.</p>
            </td>
          </tr>
          <tr>
            <td align="center" style="padding:28px 40px 8px;">
              <a href="{{link}}" style="display:inline-block;padding:13px 32px;background-color:#5B3CD9;color:#FAFAFA;border-radius:8px;text-decoration:none;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:15px;font-weight:600;">Review my account</a>
            </td>
          </tr>
          <tr>
            <td style="padding:28px 40px 32px;border-top:1px solid #E4E4E7;">
              <p style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:12px;line-height:1.6;color:#A1A1AA;">If this was you, no action is needed. If you do not recognize this sign-in, reset your password immediately. This is an automated message, please do not reply.</p>
            </td>
          </tr>
        </table>
        <table role="presentation" width="600" cellpadding="0" cellspacing="0" style="max-width:600px;">
          <tr>
            <td align="center" style="padding:20px 16px 0;">
              <p style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:12px;color:#A1A1AA;">Notifi &mdash; notifications, delivered.</p>
            </td>
          </tr>
        </table>
      </td>
    </tr>
  </table>
</body>
</html>
"##;

/// Byte-for-byte transplant of the original `assets/templates/member-invite.html`.
const MEMBER_INVITE_HTML: &str = r##"
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta name="color-scheme" content="light">
  <meta name="supported-color-schemes" content="light">
  <title>You've been invited to {{project}} on Notifi</title>
</head>
<body style="margin:0;padding:0;background-color:#F5F5F5;">
  <div style="display:none;max-height:0;overflow:hidden;opacity:0;">
    You've been invited to join {{project}} on Notifi as {{role}}. Accept within 7 days.
  </div>
  <table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="background-color:#F5F5F5;">
    <tr>
      <td align="center" style="padding:40px 16px;">
        <table role="presentation" width="600" cellpadding="0" cellspacing="0" style="max-width:600px;background-color:#ffffff;border-radius:12px;overflow:hidden;border:1px solid #E4E4E7;">
          <tr>
            <td style="padding:32px 40px 0;">
              <p style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:20px;font-weight:700;letter-spacing:-0.02em;color:#5B3CD9;">Notifi</p>
            </td>
          </tr>
          <tr>
            <td style="padding:24px 40px 0;">
              <h1 style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:24px;font-weight:700;letter-spacing:-0.01em;color:#0A0A0A;">You've been invited to {{project}}</h1>
              <p style="margin:12px 0 0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:15px;line-height:1.6;color:#52525B;">Hi {{name}},</p>
              <p style="margin:12px 0 0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:15px;line-height:1.6;color:#52525B;">You've been invited to join the <strong>{{project}}</strong> project on Notifi as <strong>{{role}}</strong>. Accept within 7 days to join the team — you'll be asked to sign in first if you aren't already.</p>
            </td>
          </tr>
          <tr>
            <td align="center" style="padding:28px 40px 8px;">
              <a href="{{link}}" style="display:inline-block;padding:13px 32px;background-color:#5B3CD9;color:#FAFAFA;border-radius:8px;text-decoration:none;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:15px;font-weight:600;">Review invitation</a>
            </td>
          </tr>
          <tr>
            <td style="padding:16px 40px 0;">
              <p style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:13px;line-height:1.6;color:#71717A;">Button not working? Paste this link into your browser:</p>
              <p style="margin:8px 0 0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:13px;line-height:1.6;word-break:break-all;"><a href="{{link}}" style="color:#5B3CD9;">{{link}}</a></p>
            </td>
          </tr>
          <tr>
            <td style="padding:28px 40px 32px;border-top:1px solid #E4E4E7;">
              <p style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:12px;line-height:1.6;color:#A1A1AA;">If you were not expecting this invitation, you can safely ignore this email. This is an automated message, please do not reply.</p>
            </td>
          </tr>
        </table>
        <table role="presentation" width="600" cellpadding="0" cellspacing="0" style="max-width:600px;">
          <tr>
            <td align="center" style="padding:20px 16px 0;">
              <p style="margin:0;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;font-size:12px;color:#A1A1AA;">Notifi &mdash; notifications, delivered.</p>
            </td>
          </tr>
        </table>
      </td>
    </tr>
  </table>
</body>
</html>
"##;

/// A built-in system template: the code-owned default body. The store's
/// copy — seeded from here when missing — always wins at render time;
/// built-ins only fill absences, never overwrite edits.
struct BuiltIn {
    name: &'static str,
    subject: &'static str,
    html: &'static str,
}

impl BuiltIn {
    fn get(name: &str) -> Option<&'static Self> {
        BUILT_INS.iter().find(|b| b.name == name)
    }

    fn key(&self) -> String {
        format!("{TEMPLATES_FOLDER}/{}.html", self.name)
    }
}

const BUILT_INS: &[BuiltIn] = &[
    BuiltIn {
        name: "verify-email",
        subject: "Verify your Notifi account",
        html: VERIFY_EMAIL_HTML,
    },
    BuiltIn {
        name: "password-reset",
        subject: "Reset your Notifi password",
        html: PASSWORD_RESET_HTML,
    },
    BuiltIn {
        name: "password-changed",
        subject: "Your Notifi password was changed",
        html: PASSWORD_CHANGED_HTML,
    },
    BuiltIn {
        name: "login-notice",
        subject: "New sign-in to your Notifi account",
        html: LOGIN_NOTICE_HTML,
    },
    BuiltIn {
        name: "member-invite",
        subject: "You've been invited to {{project}} on Notifi",
        html: MEMBER_INVITE_HTML,
    },
];

/// Plaintext fallback: crude tag strip — fine for system mail.
fn strip_tags(html: &str) -> String {
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
}

/// Substitutes `{{key}}` placeholders via the shared kernel engine (one
/// fill implementation for system mail and channel templates alike).
fn substitute(template: &str, vars: &HashMap<String, String>) -> String {
    crate::kernel::templating::fill(template, vars)
}

/// The store-backed reader: Redis forever-keys answer every render when
/// Redis is configured; the store only answers cold names (once). With
/// no Redis, the same cache lives in-process instead.
pub struct StoreSystemTemplates {
    store: Arc<dyn ConfigStore + Send + Sync>,
    cache: Option<Arc<dyn CacheBackend + Send + Sync>>,
    warm: Mutex<HashMap<String, String>>,
}

impl StoreSystemTemplates {
    pub fn new(
        store: Arc<dyn ConfigStore + Send + Sync>,
        cache: Option<Arc<dyn CacheBackend + Send + Sync>>,
    ) -> Self {
        Self {
            store,
            cache,
            warm: Mutex::new(HashMap::new()),
        }
    }

    /// Boot pass: upload built-ins the folder is missing (an edited file
    /// is never clobbered), then read every `*.html` into the cache.
    /// Returns the warmed count. A failure logs loud at the call site
    /// and leaves renders to the lazy path, which self-heals once the
    /// store is reachable again.
    pub async fn seed_and_warm(&self) -> Result<usize, String> {
        self.seed_built_ins().await?;
        self.warm_from_store().await
    }

    /// Uploads built-ins the folder doesn't have yet — the same move
    /// that put `projects/` and `backup/` in the bucket: defaults are
    /// code, storage holds no ghost copy, seeding fills absences only.
    async fn seed_built_ins(&self) -> Result<(), String> {
        let keys = self.store.list_keys(TEMPLATES_FOLDER).await?;
        for template in BUILT_INS {
            let key = template.key();
            if !keys.contains(&key) {
                self.store
                    .put_raw(&key, template.html.as_bytes().to_vec())
                    .await?;
            }
        }
        Ok(())
    }

    /// Re-reads the folder into the cache (boot + the admin re-warm
    /// endpoint): every `*.html` body overwrites its cached copy, so
    /// whatever is in the bucket now is what renders until the next
    /// warm. Returns the warmed count.
    pub async fn warm_from_store(&self) -> Result<usize, String> {
        let keys = self.store.list_keys(TEMPLATES_FOLDER).await?;
        let mut warmed = 0;
        for key in keys {
            let Some(name) = key
                .strip_prefix(&format!("{TEMPLATES_FOLDER}/"))
                .and_then(|n| n.strip_suffix(".html"))
            else {
                continue;
            };
            let body = self.store.load_raw(&key).await?;
            let body =
                String::from_utf8(body).map_err(|_| format!("template '{key}' is not utf-8"))?;
            self.stash(name, body).await;
            warmed += 1;
        }
        Ok(warmed)
    }

    /// Caches a body for good: the Redis forever-key when Redis is on,
    /// the in-process map otherwise.
    async fn stash(&self, name: &str, body: String) {
        if let Some(cache) = &self.cache {
            cache
                .set(
                    &format!("{TEMPLATES_FOLDER}/{name}.html"),
                    body.into_bytes(),
                )
                .await;
        } else {
            self.warm.lock().unwrap().insert(name.to_string(), body);
        }
    }

    /// The warm body for `name` — cache first, one lazy store fetch on
    /// a miss (which is then cached for good).
    async fn load_html(&self, name: &str) -> Result<String, MailError> {
        let key = format!("{TEMPLATES_FOLDER}/{name}.html");
        let cached = if let Some(cache) = &self.cache {
            cache.get(&key).await
        } else {
            self.warm
                .lock()
                .unwrap()
                .get(name)
                .cloned()
                .map(String::into_bytes)
        };
        if let Some(bytes) = cached {
            return String::from_utf8(bytes)
                .map_err(|_| MailError::Template(format!("template '{name}' is not utf-8")));
        }

        let body = self
            .store
            .load_raw(&key)
            .await
            .map_err(|e| MailError::Template(format!("template '{name}' unavailable: {e}")))?;
        let body = String::from_utf8(body)
            .map_err(|_| MailError::Template(format!("template '{name}' is not utf-8")))?;
        self.stash(name, body.clone()).await;
        Ok(body)
    }
}

impl SystemTemplates for StoreSystemTemplates {
    fn render<'a>(
        &'a self,
        name: &'a str,
        vars: &'a [(&'a str, &'a str)],
    ) -> BoxFut<'a, Result<RenderedTemplate, MailError>> {
        Box::pin(async move {
            let path = std::path::Path::new(name);
            if path.components().count() != 1 {
                return Err(MailError::Template(format!(
                    "invalid template name: {name}"
                )));
            }

            // Subjects are code: one template, one purpose. A body in
            // the store without a registered subject is a wiring bug —
            // add the entry to `BUILT_INS` (name, subject) and its call
            // site, then restart or re-warm.
            let subject = BuiltIn::get(name).map(|b| b.subject).ok_or_else(|| {
                MailError::Template(format!(
                    "unknown system template: {name} — register its subject in code"
                ))
            })?;

            let html = self.load_html(name).await?;
            let text = strip_tags(&html);

            let vars: HashMap<String, String> = vars
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            Ok(RenderedTemplate {
                subject: substitute(subject.trim(), &vars),
                html: substitute(&html, &vars),
                text: substitute(&text, &vars),
            })
        })
    }

    fn warm(&self) -> BoxFut<'_, Result<usize, String>> {
        Box::pin(async move { self.warm_from_store().await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::FileConfigStore;

    fn tmp_root(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "notifi-system-templates-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn reader(root: &std::path::Path) -> StoreSystemTemplates {
        reader_with_cache(root, None)
    }

    fn reader_with_cache(
        root: &std::path::Path,
        cache: Option<Arc<dyn CacheBackend + Send + Sync>>,
    ) -> StoreSystemTemplates {
        StoreSystemTemplates::new(
            Arc::new(FileConfigStore::new(root)) as Arc<dyn ConfigStore + Send + Sync>,
            cache,
        )
    }

    fn redis_like() -> Arc<dyn CacheBackend + Send + Sync> {
        // The memory backend stands in for Redis in tests; a long TTL
        // behaves like forever keys for a test's lifetime.
        Arc::new(crate::infra::config_cache::MemoryCacheBackend::new(
            std::time::Duration::from_secs(3600),
        ))
    }

    #[tokio::test]
    async fn boot_seeds_built_ins_and_renders_from_the_store() {
        let root = tmp_root("seed");
        let r = reader(&root);
        assert_eq!(r.seed_and_warm().await.unwrap(), 5);

        let rendered = r
            .render("verify-email", &[("name", "Ada"), ("link", "https://x/y")])
            .await
            .unwrap();
        assert_eq!(rendered.subject, "Verify your Notifi account");
        assert!(rendered.html.contains("Verify your email address"));
        assert!(rendered.html.contains("https://x/y"));
        assert!(!rendered.html.contains("{{link}}"));
        assert!(rendered.text.contains("Ada"));
    }

    #[tokio::test]
    async fn the_store_copy_wins_and_seeding_never_clobbers() {
        let root = tmp_root("edit");
        let r = reader(&root);
        r.store
            .put_raw(
                "templates/verify-email.html",
                b"<p>EDITED {{name}}</p>".to_vec(),
            )
            .await
            .unwrap();

        r.seed_and_warm().await.unwrap();

        let rendered = r.render("verify-email", &[("name", "Ada")]).await.unwrap();
        assert!(rendered.html.contains("EDITED Ada"));

        // The edited file survived the seed pass byte-for-byte.
        assert_eq!(
            r.store
                .load_raw("templates/verify-email.html")
                .await
                .unwrap(),
            b"<p>EDITED {{name}}</p>".to_vec()
        );
    }

    #[tokio::test]
    async fn a_cold_name_fetches_lazily_then_sticks() {
        let root = tmp_root("cold");
        let r = reader(&root);
        // Empty folder, subject registered: the render fails loudly...
        assert!(r.render("verify-email", &[]).await.is_err());
        // ...then the file lands in the bucket — no restart, no warm:
        r.store
            .put_raw(
                "templates/verify-email.html",
                b"<p>LATE {{link}}</p>".to_vec(),
            )
            .await
            .unwrap();
        let rendered = r
            .render("verify-email", &[("link", "https://x/y")])
            .await
            .unwrap();
        assert!(rendered.html.contains("LATE https://x/y"));
    }

    #[tokio::test]
    async fn unknown_name_and_traversal_are_rejected() {
        let root = tmp_root("guard");
        let r = reader(&root);
        assert!(matches!(
            r.render("nope", &[]).await,
            Err(MailError::Template(_))
        ));
        assert!(matches!(
            r.render("../secret", &[]).await,
            Err(MailError::Template(_))
        ));
    }

    #[tokio::test]
    async fn invite_subject_fills_its_project_var() {
        let root = tmp_root("invite");
        let r = reader(&root);
        r.seed_and_warm().await.unwrap();
        let rendered = r
            .render(
                "member-invite",
                &[
                    ("name", "Ada"),
                    ("project", "Acme"),
                    ("role", "developer"),
                    ("link", "https://x/i"),
                ],
            )
            .await
            .unwrap();
        assert_eq!(rendered.subject, "You've been invited to Acme on Notifi");
        assert!(!rendered.html.contains("{{"));
    }

    #[tokio::test]
    async fn renders_answer_from_the_cache_without_touching_the_store() {
        // Pre-seeded Redis, empty store: the render must come from the
        // cache alone — the bucket sees nothing at render time.
        let root = tmp_root("redis-first");
        let cache = redis_like();
        cache
            .set(
                "templates/verify-email.html",
                b"<p>FROM-REDIS {{name}}</p>".to_vec(),
            )
            .await;
        let r = reader_with_cache(&root, Some(cache));
        let rendered = r.render("verify-email", &[("name", "Ada")]).await.unwrap();
        assert!(rendered.html.contains("FROM-REDIS Ada"));
    }

    #[tokio::test]
    async fn a_warm_pass_overwrites_the_cached_copy() {
        // The "you edited the file in R2" flow: the stale forever-key is
        // replaced by the warm pass (boot or POST /admin/templates/warm),
        // and the fresh body is what sticks in the cache.
        let root = tmp_root("redis-warm");
        let cache = redis_like();
        cache
            .set("templates/verify-email.html", b"<p>STALE</p>".to_vec())
            .await;

        let r = reader_with_cache(&root, Some(Arc::clone(&cache)));
        r.store
            .put_raw(
                "templates/verify-email.html",
                b"<p>FRESH {{name}}</p>".to_vec(),
            )
            .await
            .unwrap();
        assert_eq!(r.warm_from_store().await.unwrap(), 1);

        let rendered = r.render("verify-email", &[("name", "Ada")]).await.unwrap();
        assert!(rendered.html.contains("FRESH Ada"));
        // The cache now holds the fresh bytes for every other instance.
        assert_eq!(
            cache.get("templates/verify-email.html").await.unwrap(),
            b"<p>FRESH {{name}}</p>".to_vec()
        );
    }
}
