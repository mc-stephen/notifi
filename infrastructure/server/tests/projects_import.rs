//! Project import: the export's admin-run twin (domain level). The
//! suite drives real services end to end — export one project's ZIP,
//! re-apply it as a fresh clone, and check every promise the import
//! makes (fresh tokens, remapped template ids, paused schedules, clean
//! ledger, re-keyed files) plus the manifest gatekeeping and the
//! partial-import warning semantics.

use std::io::Write;
use std::sync::Arc;

mod common;

use serde_json::{Value, json};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::common::{
    FakeApiKeysStore, FakeAuditStore, FakeAuthStore, FakeChannelProviderStore, FakeFlowStore,
    FakeHttpRequester, FakeInboundProbeStore, FakeRecipientsStore, FakeScheduleStore,
    FakeSendStore, FakeTemplatesStore, FakeTriggerTokenStore,
};
use server::domain::admin::entities::{AdminStatus, AdminUser, AdminUserId};
use server::domain::api_keys::ApiKeyService;
use server::domain::audit::AuditService;
use server::domain::auth::entities::{User, UserId, UserStatus};
use server::domain::auth::value_objects::Email;
use server::domain::error::AppError;
use server::domain::flows::FlowService;
use server::domain::inbound::InboundService;
use server::domain::projects::{ProjectExportService, ProjectImportService, ProjectService};
use server::domain::recipients::RecipientService;
use server::domain::schedules::ScheduleService;
use server::domain::schedules::entities::{ScheduleInput, ScheduleKind};
use server::domain::templates::TemplateService;
use server::infra::FileConfigStore;
use server::ports::ProjectsStore;
use server::ports::api_keys_store::ApiKeysStore;
use server::ports::auth_store::AuthStore;
use server::ports::config_store::ConfigStore;
use server::ports::{
    AdapterFactory, BoxSendFuture, DeliveryError, DeliveryMessage, DeliveryProvider,
    DeliveryReceipt,
};

struct Ctx {
    owner: UserId,
    projects: Arc<ProjectService>,
    export: Arc<ProjectExportService>,
    import: Arc<ProjectImportService>,
    flows: Arc<FlowService>,
    schedules: Arc<ScheduleService>,
    templates: Arc<TemplateService>,
    recipients: Arc<RecipientService>,
    api_keys: Arc<ApiKeyService>,
    config_store: Arc<FileConfigStore>,
    // Raw fakes — the import test grants visibility per store (the
    // in-memory fakes keep per-store registries; real stores share one
    // membership table).
    flow_store: Arc<FakeFlowStore>,
    schedule_store: Arc<FakeScheduleStore>,
    template_store: Arc<FakeTemplatesStore>,
    recipients_store: Arc<FakeRecipientsStore>,
    api_keys_store: Arc<FakeApiKeysStore>,
    send_store: Arc<FakeSendStore>,
    token_store: Arc<FakeTriggerTokenStore>,
}

impl Ctx {
    /// Grants the owner access to `project_id` in every fake registry
    /// (the services' `check_visible` asks their own store).
    fn grant(&self, project_id: &str) {
        let user = self.owner.to_string();
        self.flow_store.seed_visible(&user, project_id);
        self.schedule_store.seed_visible(&user, project_id);
        self.template_store.seed_visible(&user, project_id);
        self.recipients_store.seed_visible(&user, project_id);
        self.api_keys_store.seed_visible(&user, project_id);
        self.send_store.seed_visible(&user, project_id);
    }
}

/// One owner, one shared fake community, every service the import
/// touches — wired exactly like compose does (same stores so reads and
/// writes agree).
async fn fixture() -> Ctx {
    let auth_store = Arc::new(FakeAuthStore::new());
    let audit = Arc::new(AuditService::new(Arc::new(FakeAuditStore::new())));

    let owner = UserId::new();
    let now = chrono::Utc::now();
    auth_store.seed_user(User {
        id: owner,
        name: "Pat Doe".into(),
        email: Email::parse("pat@example.com").unwrap(),
        password_hash: String::new(),
        avatar_url: None,
        email_verified_at: Some(now),
        oauth_provider: None,
        oauth_subject: None,
        totp_secret: None,
        totp_enabled: false,
        status: UserStatus::Active,
        created_at: now,
        last_login_at: Some(now),
    });

    let root = temp_root();
    let config_store = Arc::new(FileConfigStore::new(&root));

    // One fake instance per port — every service, the export and the
    // import all share it (the shared instance is the Postgres
    // analogue: one backend, row-level shared state).
    let flow_store = Arc::new(FakeFlowStore::new());
    let schedule_store = Arc::new(FakeScheduleStore::new(flow_store.clone()));
    let template_store = Arc::new(FakeTemplatesStore::new());
    let recipients_store = Arc::new(FakeRecipientsStore::new());
    let api_keys_store = Arc::new(FakeApiKeysStore::new());
    let send_store = Arc::new(FakeSendStore::new());
    let channel_providers = Arc::new(FakeChannelProviderStore::new());
    let http = Arc::new(FakeHttpRequester::new());
    let token_store = Arc::new(FakeTriggerTokenStore::new());
    let inbound = Arc::new(InboundService::new(
        token_store.clone(),
        Arc::new(FakeInboundProbeStore::new()),
        send_store.clone(),
        template_store.clone() as Arc<dyn server::ports::templates_store::TemplatesStore>,
        channel_providers.clone(),
        Arc::new(StubFactory::succeeding()),
        http.clone(),
    ));

    let projects = Arc::new(ProjectService::new(
        auth_store.clone() as Arc<dyn ProjectsStore>,
        audit.clone(),
    ));
    let recipients = Arc::new(RecipientService::new(
        recipients_store.clone() as Arc<dyn server::ports::recipients_store::RecipientsStore>,
        audit.clone(),
    ));
    let templates = Arc::new(TemplateService::new(
        template_store.clone() as Arc<dyn server::ports::templates_store::TemplatesStore>,
        audit.clone(),
    ));
    let api_keys = Arc::new(ApiKeyService::new(
        api_keys_store.clone() as Arc<dyn ApiKeysStore>,
        audit.clone(),
    ));
    let flows = Arc::new(FlowService::new(flow_store.clone()));
    let schedules = Arc::new(ScheduleService::new(
        schedule_store.clone() as Arc<dyn server::ports::schedule_store::ScheduleStore>,
        flow_store.clone(),
        send_store.clone(),
        template_store.clone(),
        channel_providers.clone(),
        Arc::new(StubFactory::succeeding()),
        http.clone(),
    ));
    let export = Arc::new(
        ProjectExportService::new(
            auth_store.clone() as Arc<dyn ProjectsStore>,
            template_store.clone() as Arc<dyn server::ports::templates_store::TemplatesStore>,
            api_keys_store.clone() as Arc<dyn ApiKeysStore>,
            send_store.clone() as Arc<dyn server::ports::send_store::SendStore>,
            recipients_store.clone() as Arc<dyn server::ports::recipients_store::RecipientsStore>,
            flow_store.clone() as Arc<dyn server::ports::flow_store::FlowStore>,
            schedule_store.clone() as Arc<dyn server::ports::schedule_store::ScheduleStore>,
        )
        .with_config_store(
            config_store.clone() as Arc<dyn server::ports::config_store::ConfigStore + Send + Sync>
        ),
    );

    // The visibility grant: the import creates the clone mid-run (its
    // id is unknowable in advance), so the fakes need someone to push
    // (owner, project) into their registries at that moment.
    type Seed = dyn Fn(&str, &str) + Send + Sync;
    let seed: Arc<Seed> = {
        let flow = flow_store.clone();
        let schedule = schedule_store.clone();
        let templates = template_store.clone();
        let recipients = recipients_store.clone();
        let keys = api_keys_store.clone();
        let sends = send_store.clone();
        Arc::new(move |user, project| {
            flow.seed_visible(user, project);
            schedule.seed_visible(user, project);
            templates.seed_visible(user, project);
            recipients.seed_visible(user, project);
            keys.seed_visible(user, project);
            sends.seed_visible(user, project);
        })
    };

    let import = Arc::new(
        ProjectImportService::new(
            projects.clone(),
            templates.clone(),
            schedules.clone(),
            flows.clone(),
            api_keys.clone(),
            recipients.clone(),
            Some(inbound.clone()),
            auth_store.clone() as Arc<dyn AuthStore>,
            audit.clone(),
        )
        .with_visibility_seed(seed)
        .with_config_store(
            config_store.clone() as Arc<dyn server::ports::config_store::ConfigStore + Send + Sync>
        ),
    );

    Ctx {
        owner,
        projects,
        export,
        import,
        flows,
        schedules,
        templates,
        recipients,
        api_keys,
        config_store,
        flow_store,
        schedule_store,
        template_store,
        recipients_store,
        api_keys_store,
        send_store,
        token_store,
    }
}

fn temp_root() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("notifi-import-{}", server::kernel::Ulid::new()));
    std::fs::remove_dir_all(&root).ok();
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn admin() -> AdminUser {
    let now = chrono::Utc::now();
    AdminUser {
        id: AdminUserId::new(),
        name: "Root".into(),
        email: Email::parse("root@notifi.dev").unwrap(),
        password_hash: String::new(),
        totp_secret: None,
        totp_enabled: false,
        is_super_admin: true,
        status: AdminStatus::Active,
        created_at: now,
        last_login_at: None,
    }
}

/// The webhook trigger's token inside a definition — the token rides
/// the tree, so tests read it back out.
fn trigger_webhook_token(definition: &Value) -> Option<String> {
    definition["nodes"].as_array()?.iter().find(|n| {
        n["type"].as_str() == Some("trigger") && n["data"]["kind"].as_str() == Some("webhook")
    })?["data"]["token"]
        .as_str()
        .map(str::to_owned)
}

#[tokio::test]
async fn round_trip_clones_everything_and_regenerates_owners() {
    let ctx = fixture().await;

    let project = ctx
        .projects
        .create(ctx.owner, "Roundtrip", None)
        .await
        .unwrap();
    ctx.grant(&project.id);

    // Template with one template node bound into the webhook's flow.
    let template = ctx
        .templates
        .create(
            ctx.owner,
            &project.id,
            "Welcome email",
            None,
            "email",
            json!({ "subject": "Hi", "html": "<p>hello</p>" }),
            vec![],
        )
        .await
        .unwrap();
    // The webhook flow is definition-born now: one webhook trigger with
    // the builder's minted token + a bound template node.
    const WEBHOOK_TOKEN: &str = "aabbccddeeff00112233445566778899";
    let webhook_flow = ctx
        .flows
        .create(
            ctx.owner,
            &project.id,
            "Hooks",
            Some(json!({
                "version": 1,
                "nodes": [
                    { "id": "trigger", "type": "trigger",
                      "data": { "kind": "webhook", "token": WEBHOOK_TOKEN, "events": ["user.signed_up"] } },
                    { "id": "node-1", "type": "template",
                      "data": { "templateId": template.id, "channel": "email" } }
                ],
                "edges": [],
            })),
        )
        .await
        .unwrap();
    // The token rides the definition (nothing mints server-side spins).
    assert_eq!(
        trigger_webhook_token(&webhook_flow.definition).as_deref(),
        Some(WEBHOOK_TOKEN)
    );

    // A recipient the schedule names.
    ctx.recipients
        .create(
            ctx.owner,
            &project.id,
            "user-1",
            "Lena",
            json!({ "email": "lena@example.com" }),
        )
        .await
        .unwrap();

    // A recurring schedule with its own seeded flow.
    let schedule = ctx
        .schedules
        .create(
            ctx.owner,
            &project.id,
            ScheduleInput {
                name: "Weekly digest".into(),
                kind: ScheduleKind::Recurring,
                run_at: None,
                interval_secs: Some(3600),
                days: vec![],
                run_time: None,
                timezone: "UTC".into(),
                channel: "email".into(),
                title: "Digest".into(),
                body: "Here is your digest".into(),
                recipients: vec!["user-1".into()],
            },
        )
        .await
        .unwrap();
    assert!(schedule.active, "fresh schedules arm themselves");

    // A flow-page flow nobody owns. No fixture handle needed — the test
    // later looks it up by name in the clone.
    ctx.flows
        .create(
            ctx.owner,
            &project.id,
            "Standalone",
            Some(json!({
                "version": 1,
                "nodes": [{ "id": "trigger", "type": "trigger", "data": { "kind": "event" } }],
                "edges": [],
            })),
        )
        .await
        .unwrap();

    // One key (its raw value exists only here) and one storage file.
    let (_key, old_raw) = ctx
        .api_keys
        .create(ctx.owner, &project.id, "prod key", &[], None)
        .await
        .unwrap();
    let file_key = format!("projects/{}/config/email/channels.json", project.slug);
    ctx.config_store
        .put_raw(&file_key, br#"{ "host": "smtp.x" }"#.to_vec())
        .await
        .unwrap();

    // -- export ---------------------------------------------------------
    let bundle = ctx
        .export
        .build_zip(ctx.owner, &project.id)
        .await
        .expect("export succeeds");
    assert!(bundle.filename.ends_with(".zip"));

    // -- import (the original is alive, so the clone is a second
    //    project; the fake store dedupes slugs instead of conflicting —
    //    the same uniqueness seam the PG store enforces on names) ------
    let report = ctx
        .import
        .import_zip(&admin(), ctx.owner, &bundle.bytes)
        .await
        .expect("import succeeds");
    let new_project_id = report["project"]["id"].as_str().unwrap().to_string();
    assert_ne!(new_project_id, project.id, "fresh project row");
    assert_ne!(
        report["project"]["slug"], project.slug,
        "unique slug enforced"
    );
    let decrypted = report["apiKeySecrets"]
        .as_array()
        .expect("fresh secret list");
    assert_eq!(decrypted.len(), 1, "exactly one fresh secret: {report}");
    assert_ne!(
        decrypted[0]["key"].as_str().unwrap(),
        old_raw,
        "api key secrets are regenerated, never copied"
    );
    // Section counts.
    assert_eq!(report["applied"]["templates"], 1);
    assert_eq!(report["applied"]["recipients"], 1);
    assert_eq!(report["applied"]["schedules"], 1);
    // The webhook flow is definition-born now — it restores as a plain
    // flow alongside the standalone one.
    assert_eq!(report["applied"]["flows"], 2);
    assert_eq!(report["applied"]["apiKeys"], 1);
    assert_eq!(report["applied"]["projectFiles"], 1);
    // History and members are reference-only.
    assert_eq!(report["skipped"]["sendsCount"], 0);
    assert!(
        report["warnings"].as_array().unwrap().is_empty(),
        "{report}"
    );

    // -- the clone's shapes ---------------------------------------------
    // Tokens ride their definitions — the clone keeps the ORIGINAL
    // inbound token, so the webhook URL survives the project move.
    let new_flow = ctx
        .flows
        .list(ctx.owner, &new_project_id)
        .await
        .unwrap()
        .into_iter()
        .find(|f| f.name == "Hooks")
        .expect("webhook flow restored");
    let new_token =
        trigger_webhook_token(&new_flow.definition).expect("clone's trigger carries a token");
    assert_eq!(new_token, WEBHOOK_TOKEN, "webhook tokens are preserved");

    // Template nodes point at the NEW template row (old ids die with
    // the export).
    let new_templates = ctx
        .templates
        .list(ctx.owner, &new_project_id, None, i64::MAX, None)
        .await
        .unwrap();
    assert_eq!(new_templates.len(), 1);
    let new_template_id = new_templates[0].id.clone();
    assert_ne!(new_template_id, template.id);
    let definition = serde_json::to_string(&new_flow.definition).unwrap();
    assert!(
        definition.contains(&new_template_id),
        "definition remapped to the new template id"
    );
    assert!(
        !definition.contains(&template.id),
        "no stale template id survives"
    );

    // Schedules arrive paused — arming is a deliberate human act.
    let new_schedules = ctx
        .schedules
        .list(ctx.owner, &new_project_id)
        .await
        .unwrap();
    assert_eq!(new_schedules.len(), 1);
    assert!(!new_schedules[0].active, "imported schedules start paused");

    // The token indexed for the clone's /v1/hooks door (the import
    // syncs webhook-carrying definitions).
    assert!(
        ctx.token_store.all().iter().any(|t| t == &new_token),
        "clone's webhook token indexed for inbound fire"
    );

    // The flow-page flow crossed over too.
    let new_flows = ctx.flows.list(ctx.owner, &new_project_id).await.unwrap();
    assert!(
        new_flows.iter().any(|f| f.name == "Standalone"),
        "flow-page flow restored: {:?}",
        new_flows.iter().map(|f| &f.name).collect::<Vec<_>>()
    );

    // Files re-keyed under the new slug.
    let keys = ctx
        .config_store
        .list_keys(&format!(
            "projects/{}/",
            report["project"]["slug"].as_str().unwrap()
        ))
        .await
        .unwrap();
    assert!(
        keys.iter()
            .any(|k| k.ends_with("config/email/channels.json")),
        "restored files: {keys:?}"
    );
}

#[tokio::test]
async fn import_refuses_a_zip_without_a_manifest() {
    let ctx = fixture().await;

    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let buffer = std::io::Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(buffer);
    zip.start_file("notifi-stray.txt", options).unwrap();
    zip.write_all(b"nothing here").unwrap();
    let bytes = zip.finish().unwrap().into_inner();

    let err = ctx
        .import
        .import_zip(&admin(), ctx.owner, &bytes)
        .await
        .unwrap_err();
    assert!(format!("{err}").contains("manifest"), "{err}");
}

#[tokio::test]
async fn import_refuses_manifest_v1() {
    let ctx = fixture().await;
    let bytes = write_zip(vec![(
        "manifest.json",
        serde_json::to_vec_pretty(&json!({
            "version": 1,
            "project": { "name": "Legacy", "slug": "legacy" },
        }))
        .unwrap(),
    )]);

    let err = ctx
        .import
        .import_zip(&admin(), ctx.owner, &bytes)
        .await
        .unwrap_err();
    assert!(format!("{err}").contains("v2"), "{err}");
}

#[tokio::test]
async fn import_demands_a_real_owner_account() {
    let ctx = fixture().await;
    let bytes = write_zip(vec![(
        "manifest.json",
        serde_json::to_vec_pretty(&json!({
            "version": 2,
            "project": { "name": "Orphaned", "slug": "orphaned" },
        }))
        .unwrap(),
    )]);

    let err = ctx
        .import
        .import_zip(&admin(), UserId::new(), &bytes)
        .await
        .unwrap_err();
    match err {
        AppError::NotFound(_) => {}
        other => panic!("expected NotFound, got {other}"),
    }
}

#[tokio::test]
async fn partial_rows_warn_instead_of_failing_the_run() {
    let ctx = fixture().await;
    let mut entries: Vec<(&'static str, Vec<u8>)> = vec![(
        "manifest.json",
        serde_json::to_vec_pretty(&json!({
            "version": 2,
            "project": { "name": "Partial", "slug": "partial" },
            "sections": { "sends": { "count": 4 }, "members": { "count": 2 } },
            "subscription": { "plan": "free" },
        }))
        .unwrap(),
    )];
    for (name, section) in [
        (
            "schedules.json",
            json!([
                { "name": "Broken", "kind": "meteor", "timezone": "UTC", "channel": "email", "title": "t", "body": "b", "recipients": [] }
            ]),
        ),
        (
            "recipients.json",
            json!([{ "name": "Keyless", "contacts": {} }]),
        ),
        ("flows.json", json!([])),
        ("templates.json", json!([])),
        ("apiKeys.json", json!([])),
    ] {
        entries.push((name, serde_json::to_vec_pretty(&section).unwrap()));
    }
    let bytes = write_zip(entries);

    let report = ctx
        .import
        .import_zip(&admin(), ctx.owner, &bytes)
        .await
        .expect("row-level failures never abort the run");
    assert_eq!(report["applied"]["schedules"], 0);
    assert_eq!(report["applied"]["recipients"], 0);
    let warnings = report["warnings"].as_array().unwrap();
    assert!(
        warnings
            .iter()
            .any(|w| w.as_str().unwrap().contains("unknown kind")),
        "{warnings:?}"
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.as_str().unwrap().contains("targeting key")),
        "{warnings:?}"
    );
    // The project itself still exists (created before the rows).
    assert_eq!(report["project"]["name"], "Partial");
    // History is transparent: the manifest's send count rides the report.
    assert_eq!(report["skipped"]["sendsCount"], 4);
}

// -- helpers -----------------------------------------------------------

/// The delivery stub every suite carries: always-or-conditionally
/// succeeds without touching a network.
struct StubAdapter {
    succeed: bool,
}

impl DeliveryProvider for StubAdapter {
    fn channel(&self) -> &'static str {
        "stub"
    }

    fn provider_name(&self) -> &'static str {
        "stub"
    }

    fn send(&self, _message: &DeliveryMessage) -> BoxSendFuture<'_> {
        let succeed = self.succeed;
        Box::pin(async move {
            if succeed {
                Ok(DeliveryReceipt {
                    provider: "stub".to_string(),
                    provider_message_id: Some("mid-1".to_string()),
                })
            } else {
                Err(DeliveryError::Provider("boom".to_string()))
            }
        })
    }
}

struct StubFactory {
    fail: std::sync::Mutex<std::collections::HashSet<String>>,
}

impl StubFactory {
    fn succeeding() -> Self {
        Self {
            fail: std::sync::Mutex::new(std::collections::HashSet::new()),
        }
    }
}

impl AdapterFactory for StubFactory {
    fn adapter(
        &self,
        _project_id: &str,
        _channel: &str,
        provider_id: &str,
        _config: &serde_json::Value,
    ) -> Option<Arc<dyn DeliveryProvider>> {
        if provider_id == "ghost" {
            return None;
        }
        let succeed = !self.fail.lock().unwrap().contains(provider_id);
        Some(Arc::new(StubAdapter { succeed }))
    }
}

/// Builds a zip from `name → bytes` entries.
fn write_zip(entries: Vec<(&'static str, Vec<u8>)>) -> Vec<u8> {
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let buffer = std::io::Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(buffer);
    for (name, bytes) in entries {
        zip.start_file(name, options).unwrap();
        zip.write_all(bytes.as_slice()).unwrap();
    }
    zip.finish().unwrap().into_inner()
}
