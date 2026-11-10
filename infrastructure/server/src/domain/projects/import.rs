//! Project import: the admin-run twin of the export (manifest v2 ZIPs).
//! Staff-only by design — the endpoint mounts under `/admin`, every run
//! names its `owner` (the account that receives the clone), and the
//! audit log records the admin behind it via [`AuditEvent::new_admin`].
//!
//! Semantics (ARCHITECTURE §24, "Import"):
//! * **Always a fresh project.** The manifest's name is reused when it
//!   is free (the exported project was hard-deleted); a live collision
//!   appends a short random suffix. Tombstoned rows are never written
//!   into — deletion runs unbothered, and a clone loses nothing critical
//!   (exact-history recovery is the backup path, not the zip).

//! * **Reference-only sections:** sends (the clone starts a clean
//!   ledger), members (the `owner` field attributes the clone),
//!   subscription (billing is per project — a clone starts with the
//!   billing state any new project gets; a paid plan must never clone
//!   itself by import).
//! * **Partial-import semantics:** row-level failures never abort the
//!   run — they land in `warnings` and the counts say exactly what
//!   applied. Only the project creation itself fails loudly (nothing
//!   else has been touched at that point).
//!
//! Layer: domain (framework-free). Everything flows through the same
//! services the app surfaces use, so validations, folder materialization
//! and token sync semantics ride along — an import never hand-writes
//! store rows or config files outside these paths.

use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;

use rand::RngCore as _;
use serde_json::{Value, json};

use crate::domain::admin::entities::AdminUser;
use crate::domain::api_keys::ApiKeyService;
use crate::domain::audit::AuditService;
use crate::domain::audit::entities::{AuditAction, AuditEvent};
use crate::domain::auth::entities::UserId;
use crate::domain::error::AppError;
use crate::domain::flows::FlowService;
use crate::domain::inbound::InboundService;
use crate::domain::recipients::RecipientService;
use crate::domain::schedules::ScheduleService;
use crate::domain::schedules::entities::{ScheduleInput, ScheduleKind};
use crate::domain::schedules::services::SchedulePatch;
use crate::domain::templates::TemplateService;
use crate::ports::auth_store::AuthStore;
use crate::ports::config_store::ConfigStore;
use crate::ports::templates_store::AttachmentInput;

/// How many times the importer retries project creation with a suffixed
/// name before giving up (each suffix carries ~20 bits of entropy; five
/// misses without one is not bad luck, it is a bug).
const NAME_RETRIES: usize = 5;

/// Longest suffix label — `{name} (AbC12)`.
const SUFFIX_LEN: usize = 5;

/// The visibility-grant seam (see the struct field docs).
type VisibilitySeed = dyn Fn(&str, &str) + Send + Sync;

pub struct ProjectImportService {
    projects: Arc<crate::domain::projects::ProjectService>,
    templates: Arc<TemplateService>,
    schedules: Arc<ScheduleService>,
    flows: Arc<FlowService>,
    api_keys: Arc<ApiKeyService>,
    recipients: Arc<RecipientService>,
    inbound: Option<Arc<InboundService>>,
    auth: Arc<dyn AuthStore>,
    audit: Arc<AuditService>,
    /// Drive for the zip's project files: `Some` in the composed app,
    /// `None` in unit tests exercising only the row imports.
    config_store: Option<Arc<dyn ConfigStore>>,
    /// Optional visibility grant for the freshly created clone.
    ///
    /// Real stores share one membership table, so an imported project
    /// is visible the instant its row lands — compose never raises
    /// this. The test fakes keep per-store registries instead, and an
    /// import creates the clone mid-run (id unknowable in advance), so
    /// a fixture injects the same hand-seeding the other suites do by
    /// name (see the row-import suites' `setup_project` fixtures).
    seed_visibility: Option<Arc<VisibilitySeed>>,
}

/// The export zip, indexed: every entry as `name → bytes`.
struct ZipBundle(Vec<(String, Vec<u8>)>);

impl ZipBundle {
    fn get(&self, name: &str) -> Option<&[u8]> {
        self.0
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, bytes)| bytes.as_slice())
    }

    fn rows(&self, section: &str) -> Vec<Value> {
        serde_json::from_slice::<Value>(self.get(section).unwrap_or(&[]))
            .map(|v| v.as_array().cloned().unwrap_or_default())
            .unwrap_or_default()
    }
}

impl ProjectImportService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        projects: Arc<crate::domain::projects::ProjectService>,
        templates: Arc<TemplateService>,
        schedules: Arc<ScheduleService>,
        flows: Arc<FlowService>,
        api_keys: Arc<ApiKeyService>,
        recipients: Arc<RecipientService>,
        inbound: Option<Arc<InboundService>>,
        auth: Arc<dyn AuthStore>,
        audit: Arc<AuditService>,
    ) -> Self {
        Self {
            projects,
            templates,
            schedules,
            flows,
            api_keys,
            recipients,
            inbound,
            auth,
            audit,
            config_store: None,
            seed_visibility: None,
        }
    }

    /// Wires project storage so the zip's files re-key into the clone.
    pub fn with_config_store(mut self, config_store: Arc<dyn ConfigStore>) -> Self {
        self.config_store = Some(config_store);
        self
    }

    /// Injects the test-only visibility grant (see the field doc).
    pub fn with_visibility_seed(mut self, seed: Arc<VisibilitySeed>) -> Self {
        self.seed_visibility = Some(seed);
        self
    }

    /// Applies one exported ZIP for `admin` on behalf of `owner`. The
    /// owner (member of record afterwards) is what the project's
    /// permission checks see; the admin stays the recorded *actor*.
    pub async fn import_zip(
        &self,
        admin: &AdminUser,
        owner: UserId,
        zip_bytes: &[u8],
    ) -> Result<Value, AppError> {
        let bundle = Self::unzip(zip_bytes)?;
        let manifest: Value = serde_json::from_slice(
            bundle
                .get("manifest.json")
                .ok_or_else(|| AppError::Validation("the zip carries no manifest.json".into()))?,
        )
        .map_err(|e| AppError::Validation(format!("manifest.json is not valid JSON: {e}")))?;
        if manifest["version"] != 2 {
            return Err(AppError::Validation(
                "this import expects a manifest v2 export (re-export the project first)".into(),
            ));
        }
        let manifest_name = manifest["project"]["name"]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| AppError::Validation("the manifest carries no project name".into()))?
            .to_string();
        let manifest_slug = manifest["project"]["slug"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string();

        // The owner account must exist — the admin names the clone's
        // home explicitly (a silent "orphan project" is never an option).
        self.auth
            .find_user_by_id(owner)
            .await
            .map_err(|e| AppError::Storage(format!("owner lookup failed: {e}")))?
            .ok_or_else(|| AppError::NotFound("no account with that owner id".into()))?;

        // -- 1. the fresh project (name suffixed on live collisions) ----
        let project = {
            let mut name = manifest_name.clone();
            let mut created = None;
            for _ in 0..NAME_RETRIES {
                match self.projects.create(owner, &name, None).await {
                    Ok(project) => {
                        created = Some(project);
                        break;
                    }
                    Err(AppError::Conflict(_)) => {
                        name = format!("{} ({})", manifest_name, Self::suffix());
                    }
                    Err(e) => return Err(e),
                }
            }
            created.ok_or_else(|| {
                AppError::Conflict("no unique name could be derived from the export".into())
            })?
        };
        let project_id = project.id.clone();
        let slug = project.slug.clone();

        // Cross-store visibility for the clone (no-op under real stores).
        if let Some(seed) = &self.seed_visibility {
            seed(&owner.to_string(), &project_id);
        }

        let mut warnings: Vec<String> = vec![];
        let mut applied = json!({});

        // -- 2. project files, re-keyed under the new slug --------------
        let mut files = 0usize;
        if let Some(config_store) = self.config_store.clone()
            && !manifest_slug.is_empty()
        {
            let old_prefix = format!("projects/{manifest_slug}/");
            let new_prefix = format!("projects/{slug}/");
            for (key, bytes) in &bundle.0 {
                let Some(rest) = key.strip_prefix(&old_prefix) else {
                    continue;
                };
                if rest.is_empty() {
                    continue;
                }
                if let Err(e) = config_store
                    .put_raw(&format!("{new_prefix}{rest}"), bytes.clone())
                    .await
                {
                    warnings.push(format!("file '{key}' could not be restored: {e}"));
                } else {
                    files += 1;
                }
            }
        }
        applied["projectFiles"] = json!(files);

        // -- 3. templates (new ids; the map rewrites every definition) --
        let mut template_map: HashMap<String, String> = HashMap::new();
        for t in bundle.rows("templates.json") {
            let name = Self::row_name(&t, "Restored template");
            let old_id = t["id"].as_str().unwrap_or("").to_string();
            let attachments = t["attachments"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|a| Self::attachment_input(a, &manifest_slug, &slug))
                .collect::<Vec<_>>();
            match self
                .templates
                .create(
                    owner,
                    &project_id,
                    &name,
                    t["description"].as_str(),
                    t["channel"].as_str().unwrap_or(""),
                    t["content"].clone(),
                    attachments,
                )
                .await
            {
                Ok(template) if !old_id.is_empty() => {
                    template_map.insert(old_id, template.id.clone());
                }
                Ok(_) => {}
                Err(e) => warnings.push(format!("template '{name}': {e}")),
            }
        }
        applied["templates"] = json!(template_map.len());
        applied["templatesRequested"] = json!(bundle.rows("templates.json").len());

        // -- 4. recipients (same in-house targeting keys) ---------------
        let mut recipients = 0usize;
        for r in bundle.rows("recipients.json") {
            let key = r["userId"].as_str().unwrap_or("").trim().to_string();
            if key.is_empty() {
                warnings.push("a recipient with no targeting key was skipped".into());
                continue;
            }
            match self
                .recipients
                .create(
                    owner,
                    &project_id,
                    &key,
                    &Self::row_name(&r, "Restored recipient"),
                    r["contacts"].clone(),
                )
                .await
            {
                Ok(_) => recipients += 1,
                Err(e) => warnings.push(format!("recipient '{key}': {e}")),
            }
        }
        applied["recipients"] = json!(recipients);

        // -- 5. definitions index (flows restore through their owners) --
        let mut definitions: HashMap<String, (String, Value)> = HashMap::new();
        for f in bundle.rows("flows.json") {
            let id = f["id"].as_str().unwrap_or("").to_string();
            if id.is_empty() {
                continue;
            }
            definitions.insert(
                id,
                (Self::row_name(&f, "Restored flow"), f["definition"].clone()),
            );
        }

        // -- 6. schedules (imported paused; nothing sends until armed) --
        let exported_schedules = bundle.rows("schedules.json");
        let mut schedules = 0usize;
        for s in &exported_schedules {
            let name = Self::row_name(s, "Restored schedule");
            let old_flow = s["flowId"].as_str().unwrap_or("").to_string();
            let kind = match s["kind"].as_str().and_then(ScheduleKind::parse) {
                Some(kind) => kind,
                None => {
                    warnings.push(format!("schedule '{name}': unknown kind — skipped"));
                    continue;
                }
            };
            let run_at = s["runAt"]
                .as_str()
                .and_then(|raw| chrono::DateTime::parse_from_rfc3339(raw).ok())
                .map(|ts| ts.with_timezone(&chrono::Utc));
            match self
                .schedules
                .create(
                    owner,
                    &project_id,
                    ScheduleInput {
                        name: name.to_string(),
                        kind,
                        run_at,
                        interval_secs: s["intervalSecs"].as_i64(),
                        days: Self::i64_list(&s["days"]),
                        run_time: s["runTime"].as_str().map(str::to_owned),
                        timezone: s["timezone"].as_str().unwrap_or("UTC").to_string(),
                        channel: s["channel"].as_str().unwrap_or("").to_string(),
                        title: s["title"].as_str().unwrap_or("").to_string(),
                        body: s["body"].as_str().unwrap_or("").to_string(),
                        recipients: Self::string_list(&s["recipients"]),
                    },
                )
                .await
            {
                Ok(schedule) => {
                    let (definition, apply) = self
                        .apply_owned_definition(
                            owner,
                            &project_id,
                            &old_flow,
                            &schedule.flow_id,
                            &definitions,
                            &template_map,
                        )
                        .await;
                    if let Err(warning) = apply {
                        warnings.push(warning);
                    } else if let Some(inbound) = &self.inbound
                        && definition.is_object()
                    {
                        inbound
                            .sync_flow_tokens(&project_id, &schedule.flow_id, &definition)
                            .await;
                    }
                    // Paused regardless of the export's flag: timing rows
                    // arm themselves deliberately, never by import.
                    if let Err(e) = self
                        .schedules
                        .update(
                            owner,
                            &project_id,
                            &schedule.id,
                            SchedulePatch {
                                active: Some(false),
                                ..SchedulePatch::default()
                            },
                        )
                        .await
                    {
                        warnings.push(format!("schedule '{name}' pause: {e}"));
                    }
                    // Legacy rows bound content to a template id; that
                    // pointer cannot ride ScheduleInput — say so once.
                    if s["templateId"].is_string() {
                        warnings.push(format!(
                            "schedule '{name}': legacy template binding not restored — the flow owns content now; attach a template node if the title/body fallback is not enough"
                        ));
                    }
                    schedules += 1;
                }
                Err(e) => warnings.push(format!("schedule '{name}': {e}")),
            }
        }
        applied["schedules"] = json!(schedules);

        // -- 7. flow-page flows (everything no owner claimed) -----------
        let mut claimed: HashMap<String, ()> = HashMap::new();
        for rows in [&exported_schedules] {
            for row in rows {
                if let Some(flow_id) = row["flowId"].as_str() {
                    claimed.insert(flow_id.to_string(), ());
                }
            }
        }
        let mut flow_pages = 0usize;
        for (old_id, (name, exported)) in &definitions {
            if claimed.contains_key(old_id) {
                continue;
            }
            let mut definition = exported.clone();
            Self::remap_templates(&mut definition, &template_map);
            match self
                .flows
                .create(owner, &project_id, name, Some(definition.clone()))
                .await
            {
                Ok(flow) => {
                    // Webhook tokens ride their definitions — a fresh
                    // sync indexes them for the clone's /v1/hooks door.
                    if let Some(inbound) = &self.inbound {
                        inbound
                            .sync_flow_tokens(&project_id, &flow.id, &definition)
                            .await;
                    }
                    flow_pages += 1;
                }
                Err(e) => warnings.push(format!("flow '{name}': {e}")),
            }
        }
        applied["flows"] = json!(flow_pages);

        // -- 8. api keys (fresh secrets; revoked stay dead records) -----
        let mut keys = 0usize;
        let mut key_secrets = vec![];
        for k in bundle.rows("apiKeys.json") {
            if k["revokedAt"].is_string() {
                continue;
            }
            let name = Self::row_name(&k, "Restored key");
            let expires_at = k["expiresAt"]
                .as_str()
                .and_then(|raw| chrono::DateTime::parse_from_rfc3339(raw).ok())
                .map(|ts| ts.with_timezone(&chrono::Utc));
            match self
                .api_keys
                .create(
                    owner,
                    &project_id,
                    &name,
                    &Self::string_list(&k["permissions"]),
                    expires_at,
                )
                .await
            {
                Ok((api_key, raw)) => {
                    if !k["enabled"].as_bool().unwrap_or(true)
                        && let Err(e) = self
                            .api_keys
                            .set_enabled(owner, &project_id, &api_key.id, false)
                            .await
                    {
                        warnings.push(format!("api key '{name}' disable: {e}"));
                    }
                    key_secrets.push(json!({ "name": name, "key": raw }));
                    keys += 1;
                }
                Err(e) => warnings.push(format!("api key '{name}': {e}")),
            }
        }
        applied["apiKeys"] = json!(keys);
        applied["apiKeysRequested"] = json!(bundle.rows("apiKeys.json").len());

        // -- 9. report + audit ------------------------------------------
        let report = json!({
            "project": {
                "id": project.id,
                "name": project.name,
                "slug": project.slug,
                "originalName": manifest_name,
                "originalSlug": manifest_slug,
                "suffixed": project.name != manifest_name,
            },
            "owner": owner.to_string(),
            "applied": applied,
            "skipped": {
                "sendsCount": manifest["sections"]["sends"]["count"],
                "membersCount": manifest["sections"]["members"]["count"],
                "subscription": manifest["subscription"],
                "note": "history and billing never re-enter through an import",
            },
            "apiKeySecrets": key_secrets,
            "warnings": warnings,
        });

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new_admin(
                    AuditAction::ProjectImported,
                    &admin.id.to_string(),
                    Some(&admin.name),
                    Some(&project_id),
                    format!(
                        "project '{}' imported for owner {} from the export of '{}'",
                        project.name, owner, manifest_name
                    ),
                    Some(json!({ "applied": applied, "warnings": warnings.len() })),
                ),
            )
            .await;

        Ok(report)
    }

    /// Overwrites one owner-born flow's definition with its exported
    /// twin: template ids remapped. Trigger tokens need no re-pointing —
    /// webhook tokens ride inside the flow definitions themselves, so
    /// the clone keeps its own URLs. A definition the zip lacks (or a
    /// broken flow update) only warns: the owner row is already healthy
    /// with its seed definition.
    async fn apply_owned_definition(
        &self,
        owner: UserId,
        project_id: &str,
        old_flow_id: &str,
        new_flow_id: &str,
        definitions: &HashMap<String, (String, Value)>,
        template_map: &HashMap<String, String>,
    ) -> (Value, Result<(), String>) {
        let Some((name, exported)) = definitions.get(old_flow_id) else {
            return (
                Value::Null,
                Err(format!(
                    "flow '{old_flow_id}' is not in the export — the clone keeps its seed definition"
                )),
            );
        };
        let mut definition = exported.clone();
        Self::remap_templates(&mut definition, template_map);
        if let Err(e) = self
            .flows
            .update(owner, project_id, new_flow_id, name, &definition)
            .await
        {
            return (
                exported.clone(),
                Err(format!("flow '{name}' definition: {e}")),
            );
        }
        (definition, Ok(()))
    }

    // -- zip helpers -----------------------------------------------------

    /// Parses the zip into its indexed bundle.
    fn unzip(zip_bytes: &[u8]) -> Result<ZipBundle, AppError> {
        let cursor = std::io::Cursor::new(zip_bytes.to_vec());
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| AppError::Validation(format!("this file is not a readable zip: {e}")))?;
        let mut entries = Vec::with_capacity(archive.len());
        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| AppError::Validation(format!("zip entry read failed: {e}")))?;
            if file.is_dir() {
                continue;
            }
            let name = file.name().to_string();
            let mut bytes = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut bytes)
                .map_err(|e| AppError::Validation(format!("zip entry read failed: {e}")))?;
            entries.push((name, bytes));
        }
        Ok(ZipBundle(entries))
    }

    // -- JSON / definition helpers ---------------------------------------

    /// Rewrites every template node's `templateId` in a definition tree
    /// through the old→new map.
    fn remap_templates(value: &mut Value, map: &HashMap<String, String>) {
        match value {
            Value::Object(object) => {
                if object.get("type").and_then(Value::as_str) == Some("template")
                    && let Some(data) = object.get_mut("data")
                    && let Some(old) = data
                        .get("templateId")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                    && let Some(new) = map.get(&old)
                {
                    data["templateId"] = json!(new);
                }
                for (_key, child) in object.iter_mut() {
                    Self::remap_templates(child, map);
                }
            }
            Value::Array(items) => {
                for item in items.iter_mut() {
                    Self::remap_templates(item, map);
                }
            }
            _ => {}
        }
    }

    fn string_list(value: &Value) -> Vec<String> {
        value
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    fn i64_list(value: &Value) -> Vec<i64> {
        value
            .as_array()
            .map(|items| items.iter().filter_map(Value::as_i64).collect())
            .unwrap_or_default()
    }

    fn row_name(row: &Value, fallback: &str) -> String {
        row["name"]
            .as_str()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or(fallback)
            .to_string()
    }

    fn attachment_input(value: Value, old_slug: &str, new_slug: &str) -> AttachmentInput {
        let url = value["url"].as_str().unwrap_or("");
        // Config-store paths re-key with the project; anything else is a
        // plain external URL and stays verbatim.
        let old_prefix = format!("projects/{old_slug}/");
        let url = if !old_slug.is_empty() && url.starts_with(&old_prefix) {
            format!("projects/{new_slug}/{}", &url[old_prefix.len()..])
        } else {
            url.to_string()
        };
        AttachmentInput {
            name: value["name"].as_str().unwrap_or("").to_string(),
            mime_type: value["mimeType"].as_str().unwrap_or("").to_string(),
            size_bytes: value["sizeBytes"].as_i64().unwrap_or(0),
            url,
        }
    }

    fn suffix() -> String {
        let mut bytes = [0u8; 3];
        rand::rng().fill_bytes(&mut bytes);
        let hex = bytes.iter().map(|b| format!("{b:02X}")).collect::<String>();
        hex[..SUFFIX_LEN].to_string()
    }
}
