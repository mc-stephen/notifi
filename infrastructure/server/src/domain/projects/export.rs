//! Project data export: the user's "download my project data" — available
//! anytime from project settings, capped per day, and the same lifeline
//! during the deletion countdown. Builds an in-memory ZIP: one JSON file
//! per data section plus the project's files at their real storage
//! paths.
//!
//! Layer: domain — this module owns small port records and turns them into
//! `serde_json::Value`s by hand (explicit field lists; no new derives on
//! the port types, so a store field can never silently leak into the ZIP —
//! notably `ApiKeyRecord::key_hash`, which is deliberately absent).
//!
//! Reads lean on per-project store verbs that don't re-check visibility
//! (`list_by_project`) and on tombstone-tolerant read paths (templates,
//! keys, members) so the window is downloadable; the rate limit + audit
//! stay in [`ProjectService::guard_export`].

use std::io::Write;
use std::sync::Arc;

use serde_json::{Value, json};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::domain::auth::entities::UserId;
use crate::domain::error::AppError;
use crate::ports::api_keys_store::ApiKeysStore;
use crate::ports::config_store::ConfigStore;
use crate::ports::flow_store::FlowStore;
use crate::ports::projects_store::{ProjectsStore, TeamMemberRecord};
use crate::ports::recipients_store::{RecipientRecord, RecipientsStore};
use crate::ports::send_store::{SendRecord, SendStore};
use crate::ports::templates_store::{AttachmentRecord, TemplateRecord, TemplatesStore};
use crate::ports::webhook_store::{WebhookRecord, WebhookStore};

/// Row caps — the ZIP is built in memory, so big sections stop at a
/// documented ceiling instead of risking a project with a million sends.
const CAPS: &[(&str, i64)] = &[
    ("members", 500),
    ("sends", 500),
    ("recipients", 5_000),
    ("templates", 1_000),
    ("flows", 500),
    ("webhooks", 500),
    ("apiKeys", 200),
];

fn cap(section: &str) -> i64 {
    CAPS.iter()
        .find(|(name, _)| *name == section)
        .map(|(_, cap)| *cap)
        .expect("cap table lists every section")
}

pub struct ProjectExportService {
    projects: Arc<dyn ProjectsStore>,
    templates: Arc<dyn TemplatesStore>,
    api_keys: Arc<dyn ApiKeysStore>,
    sends: Arc<dyn SendStore>,
    recipients: Arc<dyn RecipientsStore>,
    webhooks: Arc<dyn WebhookStore>,
    flows: Arc<dyn FlowStore>,
    /// Project storage: `Some` in the composed app; `None` in unit tests
    /// exercising only the DB rows.
    config_store: Option<Arc<dyn ConfigStore>>,
}

/// A finished export: the zip bytes and the filename to ship them under.
pub struct ExportBundle {
    pub filename: String,
    pub bytes: Vec<u8>,
}

impl std::fmt::Debug for ExportBundle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExportBundle")
            .field("filename", &self.filename)
            .field("bytes.len", &self.bytes.len())
            .finish()
    }
}

impl ProjectExportService {
    pub fn new(
        projects: Arc<dyn ProjectsStore>,
        templates: Arc<dyn TemplatesStore>,
        api_keys: Arc<dyn ApiKeysStore>,
        sends: Arc<dyn SendStore>,
        recipients: Arc<dyn RecipientsStore>,
        webhooks: Arc<dyn WebhookStore>,
        flows: Arc<dyn FlowStore>,
    ) -> Self {
        Self {
            projects,
            templates,
            api_keys,
            sends,
            recipients,
            webhooks,
            flows,
            config_store: None,
        }
    }

    /// Wires project storage so the zip also carries the project's files.
    pub fn with_config_store(mut self, config_store: Arc<dyn ConfigStore>) -> Self {
        self.config_store = Some(config_store);
        self
    }

    /// Builds the full export ZIP for one project the actor belongs to
    /// (tombstones included — the countdown window must stay downloadable).
    pub async fn build_zip(
        &self,
        actor: UserId,
        project_id: &str,
    ) -> Result<ExportBundle, AppError> {
        // Authorization + slug in one read: the actor finds the project in
        // their own list (deletion tombstones included), or it doesn't
        // exist for them — no enumeration either way.
        let summary = self
            .projects
            .list_projects(actor)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .find(|p| p.id == project_id)
            .ok_or_else(|| AppError::NotFound("project not found".to_string()))?;

        let members = self
            .projects
            .list_members(actor, project_id)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(Self::member_json)
            .collect::<Vec<_>>();

        let templates = self
            .templates
            .list(actor, project_id, None, cap("templates"), None)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(Self::template_json)
            .collect::<Vec<_>>();

        let api_keys = self
            .api_keys
            .list_keys(actor, project_id)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(Self::api_key_json)
            .collect::<Vec<_>>();

        let sends = self
            .sends
            .list_by_project(project_id, cap("sends"))
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(Self::send_json)
            .collect::<Vec<_>>();

        let recipients = self
            .recipients
            .list_by_project(project_id, cap("recipients"))
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(Self::recipient_json)
            .collect::<Vec<_>>();

        let webhooks = self
            .webhooks
            .list_by_project(project_id)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(Self::webhook_json)
            .collect::<Vec<_>>();

        let flows = self
            .flows
            .list_by_project(project_id)
            .await
            .map_err(AppError::from)?
            .into_iter()
            .map(Self::flow_json)
            .collect::<Vec<_>>();

        // Project files: every object under projects/{slug}/ joins the
        // zip at its original storage path (byte-for-byte).
        let mut project_files: Vec<(String, Vec<u8>)> = Vec::new();
        if let Some(config_store) = &self.config_store {
            let prefix = crate::kernel::config::ConfigResolver::project_prefix(&summary.slug);
            let keys = config_store.list_keys(&prefix).await.map_err(|e| {
                AppError::Storage(format!("project files could not be listed: {}", e))
            })?;
            for key in keys {
                let bytes = config_store.load_raw(&key).await.map_err(|e| {
                    AppError::Storage(format!("project file could not be read: {}", e))
                })?;
                project_files.push((key, bytes));
            }
        }

        let manifest = json!({
            "version": 1,
            "generatedAt": chrono::Utc::now().to_rfc3339(),
            "project": {
                "id": summary.id,
                "name": summary.name,
                "slug": summary.slug,
                "description": summary.description,
                "active": summary.active,
                "createdAt": summary.created_at.to_rfc3339(),
                "deletedAt": summary.deleted_at.map(|t| t.to_rfc3339()),
                "purgeAfter": summary.purge_after.map(|t| t.to_rfc3339()),
            },
            "sections": {
                "members": Self::section(&members, "members"),
                "templates": Self::section(&templates, "templates"),
                "apiKeys": Self::section(&api_keys, "apiKeys"),
                "sends": Self::section(&sends, "sends"),
                "recipients": Self::section(&recipients, "recipients"),
                "webhooks": Self::section(&webhooks, "webhooks"),
                "flows": Self::section(&flows, "flows"),
                "projectFiles": {
                    "count": project_files.len(),
                    "paths": project_files.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>(),
                },
            },
        });

        // Section order in the zip: manifest first (added by the
        // assembler), then the data sections, then project files at
        // their real storage paths.
        let section_bytes = |name: &str, rows: &Value| {
            serde_json::to_vec_pretty(rows)
                .map(|bytes| (name.to_string(), bytes))
                .map_err(|e| AppError::Storage(format!("export section could not be encoded: {e}")))
        };
        let mut entries: Vec<(String, Vec<u8>)> =
            vec![section_bytes("members.json", &json!(members))?];
        entries.push(section_bytes("templates.json", &json!(templates))?);
        entries.push(section_bytes("apiKeys.json", &json!(api_keys))?);
        entries.push(section_bytes("sends.json", &json!(sends))?);
        entries.push(section_bytes("recipients.json", &json!(recipients))?);
        entries.push(section_bytes("webhooks.json", &json!(webhooks))?);
        entries.push(section_bytes("flows.json", &json!(flows))?);
        entries.extend(project_files);

        Ok(ExportBundle {
            filename: format!(
                "{}-export-{}.zip",
                summary.slug,
                chrono::Utc::now().format("%Y%m%d-%H%M%S")
            ),
            bytes: Self::assemble_zip(manifest, entries)?,
        })
    }

    /// Section metadata for the manifest: count plus the cap note.
    fn section(rows: &[Value], name: &str) -> Value {
        let limit = cap(name);
        json!({
            "count": rows.len(),
            "truncated": rows.len() == limit as usize,
            "cap": limit,
        })
    }

    fn assemble_zip(manifest: Value, entries: Vec<(String, Vec<u8>)>) -> Result<Vec<u8>, AppError> {
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let buffer = std::io::Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(buffer);

        let manifest_bytes = serde_json::to_vec_pretty(&manifest)
            .map_err(|e| AppError::Storage(format!("export manifest could not be encoded: {e}")))?;

        // Section order in the zip: manifest first, then the data
        // sections, then project files at their real storage paths.
        for (name, bytes) in
            std::iter::once(("manifest.json".to_string(), manifest_bytes)).chain(entries)
        {
            zip.start_file(name.as_str(), options)
                .map_err(|e| AppError::Storage(format!("export zip entry failed: {e}")))?;
            zip.write_all(&bytes)
                .map_err(|e| AppError::Storage(format!("export zip write failed: {e}")))?;
        }

        let cursor = zip
            .finish()
            .map_err(|e| AppError::Storage(format!("export zip could not be finished: {e}")))?;
        Ok(cursor.into_inner())
    }

    // -- record → JSON mappers (explicit fields; key_hash exists and stays out).

    fn member_json(m: TeamMemberRecord) -> Value {
        json!({
            "userId": m.user_id,
            "name": m.name,
            "email": m.email,
            "role": m.role,
            "lastActiveAt": m.last_active_at.map(|t| t.to_rfc3339()),
        })
    }

    fn attachment_json(a: AttachmentRecord) -> Value {
        json!({
            "id": a.id,
            "name": a.name,
            "mimeType": a.mime_type,
            "sizeBytes": a.size_bytes,
            "url": a.url,
        })
    }

    fn template_json(t: TemplateRecord) -> Value {
        json!({
            "id": t.id,
            "name": t.name,
            "description": t.description,
            "channel": t.channel,
            "content": t.content,
            "version": t.version,
            "attachments": t.attachments.into_iter().map(Self::attachment_json).collect::<Vec<_>>(),
            "createdAt": t.created_at.to_rfc3339(),
            "updatedAt": t.updated_at.to_rfc3339(),
        })
    }

    fn api_key_json(k: crate::ports::api_keys_store::ApiKeyRecord) -> Value {
        json!({
            "id": k.id,
            "name": k.name,
            "prefix": k.prefix,
            "permissions": k.permissions,
            "expiresAt": k.expires_at.map(|t| t.to_rfc3339()),
            "lastUsedAt": k.last_used_at.map(|t| t.to_rfc3339()),
            "enabled": k.enabled,
            "revokedAt": k.revoked_at.map(|t| t.to_rfc3339()),
            "createdAt": k.created_at.to_rfc3339(),
            "note": "the key secret itself is never exported; only the prefix identifies it",
        })
    }

    fn send_json(s: SendRecord) -> Value {
        json!({
            "id": s.id,
            "channel": s.channel,
            "recipients": s.recipients,
            "title": s.title,
            "body": s.body,
            "sendAt": s.send_at.to_rfc3339(),
            "status": s.status,
            "attempts": s.attempts,
            "provider": s.provider,
        })
    }

    fn recipient_json(r: RecipientRecord) -> Value {
        json!({
            "id": r.id,
            "userId": r.user_id,
            "name": r.name,
            "contacts": r.contacts,
            "createdAt": r.created_at.to_rfc3339(),
        })
    }

    fn webhook_json(w: WebhookRecord) -> Value {
        json!({
            "id": w.id,
            "name": w.name,
            "description": w.description,
            "flowId": w.flow_id,
            "flowName": w.flow_name,
            "events": w.events,
            "active": w.active,
            "archivedAt": w.archived_at.map(|t| t.to_rfc3339()),
            "createdAt": w.created_at.to_rfc3339(),
        })
    }

    fn flow_json(f: crate::ports::flow_store::FlowRecord) -> Value {
        json!({
            "id": f.id,
            "name": f.name,
            "definition": f.definition,
            "version": f.version,
            "createdAt": f.created_at.to_rfc3339(),
            "updatedAt": f.updated_at.to_rfc3339(),
        })
    }
}
