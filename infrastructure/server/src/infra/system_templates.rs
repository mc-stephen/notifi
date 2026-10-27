//! System email templates: bodies live in the config store
//! (`templates/{name}.html` — the R2 bucket in deployment), put there
//! by ops (dashboard drag-drop or the `upload_system_templates` example)
//! — the server never writes templates, it only reads them.
//!
//! The cache is Redis forever-keys (`cfg:templates/{name}.html`, no
//! expiry): boot reads the folder into Redis, renders answer from
//! Redis, and a cold name does one store fetch and is cached for good
//! (a template dropped into the bucket later is live without a
//! restart). A warm pass — `POST /admin/templates/warm` — re-reads the
//! folder and overwrites, so an edit in the bucket lands on demand for
//! every instance sharing the Redis. With no Redis, renders read the
//! store directly (dev only).
//!
//! Subjects are code — one template, one purpose (the old
//! `.subject.txt` files are gone). Plaintext is derived by stripping
//! tags from the HTML. `{{name}}` variables ride the shared kernel
//! engine, substituted verbatim.

use std::collections::HashMap;
use std::sync::Arc;

use crate::infra::config_cache::CacheBackend;
use crate::ports::config_store::ConfigStore;
use crate::ports::mailer::{BoxFut, MailError, RenderedTemplate, SystemTemplates};

/// Store folder holding the system template bodies (top level, next to
/// `projects/` and `backup/`).
const TEMPLATES_FOLDER: &str = "templates";

/// Subject per template — code, because one template is one purpose. A
/// body in the bucket needs an entry here (and a call site) before the
/// server will render it.
const SUBJECTS: &[(&str, &str)] = &[
    ("verify-email", "Verify your Notifi account"),
    ("password-reset", "Reset your Notifi password"),
    ("password-changed", "Your Notifi password was changed"),
    ("login-notice", "New sign-in to your Notifi account"),
    (
        "member-invite",
        "You've been invited to {{project}} on Notifi",
    ),
];

fn subject_for(name: &str) -> Option<&'static str> {
    SUBJECTS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, subject)| *subject)
}

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
/// Redis is configured; the store only answers cold names (once).
pub struct StoreSystemTemplates {
    store: Arc<dyn ConfigStore + Send + Sync>,
    cache: Option<Arc<dyn CacheBackend + Send + Sync>>,
}

impl StoreSystemTemplates {
    pub fn new(
        store: Arc<dyn ConfigStore + Send + Sync>,
        cache: Option<Arc<dyn CacheBackend + Send + Sync>>,
    ) -> Self {
        Self { store, cache }
    }

    /// Reads every `*.html` in the folder into the Redis cache (boot +
    /// the admin re-warm endpoint): each body overwrites its cached
    /// copy, so whatever is in the bucket now is what renders until the
    /// next warm. Returns the warmed count. A failure logs loud at the
    /// call site and leaves renders to the lazy path, which self-heals
    /// once the store is reachable again.
    pub async fn warm_from_store(&self) -> Result<usize, String> {
        let keys = self.store.list_keys(TEMPLATES_FOLDER).await?;
        let mut warmed = 0;
        for key in keys {
            if !key.ends_with(".html") {
                continue;
            }
            let body = self.store.load_raw(&key).await?;
            let body =
                String::from_utf8(body).map_err(|_| format!("template '{key}' is not utf-8"))?;
            if let Some(cache) = &self.cache {
                cache.set(&key, body.into_bytes()).await;
            }
            warmed += 1;
        }
        Ok(warmed)
    }

    /// The warm body for `name` — the Redis forever-key first, one lazy
    /// store fetch on a miss (which is then cached for good).
    async fn load_html(&self, name: &str) -> Result<String, MailError> {
        let key = format!("{TEMPLATES_FOLDER}/{name}.html");
        if let Some(cache) = &self.cache
            && let Some(bytes) = cache.get(&key).await
        {
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
        if let Some(cache) = &self.cache {
            cache.set(&key, body.clone().into_bytes()).await;
        }
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
            // the bucket without a subject entry here is a wiring bug —
            // add the (name, subject) row and the call site, then
            // re-warm.
            let subject = subject_for(name).ok_or_else(|| {
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
    use crate::infra::{FileConfigStore, config_cache::MemoryCacheBackend};

    fn tmp_root(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "notifi-system-templates-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
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
        Arc::new(MemoryCacheBackend::new(std::time::Duration::from_secs(
            3600,
        )))
    }

    #[tokio::test]
    async fn renders_answer_from_the_cache_without_touching_the_store() {
        // Pre-seeded cache, empty store: the render must come from the
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
        let rendered = r
            .render("verify-email", &[("name", "Ada"), ("link", "https://x/y")])
            .await
            .unwrap();
        assert_eq!(rendered.subject, "Verify your Notifi account");
        assert!(rendered.html.contains("FROM-REDIS Ada"));
        assert!(rendered.text.contains("Ada"));
    }

    #[tokio::test]
    async fn a_cold_name_fetches_lazily_then_caches() {
        let root = tmp_root("cold");
        let cache = redis_like();
        let r = reader_with_cache(&root, Some(cache.clone()));
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
        // ...and the body is cached for good.
        assert_eq!(
            cache.get("templates/verify-email.html").await.unwrap(),
            b"<p>LATE {{link}}</p>".to_vec()
        );
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
        assert_eq!(
            cache.get("templates/verify-email.html").await.unwrap(),
            b"<p>FRESH {{name}}</p>".to_vec()
        );
    }

    #[tokio::test]
    async fn warm_counts_the_folder() {
        let root = tmp_root("count");
        let r = reader_with_cache(&root, Some(redis_like()));
        assert_eq!(r.warm_from_store().await.unwrap(), 0);
        r.store
            .put_raw("templates/a.html", b"<p>a</p>".to_vec())
            .await
            .unwrap();
        r.store
            .put_raw("templates/b.html", b"<p>b</p>".to_vec())
            .await
            .unwrap();
        r.store
            .put_raw("templates/notes.txt", b"skip".to_vec())
            .await
            .unwrap();
        assert_eq!(r.warm_from_store().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn unknown_name_and_traversal_are_rejected() {
        let root = tmp_root("guard");
        let r = reader_with_cache(&root, Some(redis_like()));
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
        let r = reader_with_cache(&root, Some(redis_like()));
        r.store
            .put_raw(
                "templates/member-invite.html",
                b"<p>Hi {{name}}, join {{project}} as {{role}}: {{link}}</p>".to_vec(),
            )
            .await
            .unwrap();
        r.warm_from_store().await.unwrap();
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
}
