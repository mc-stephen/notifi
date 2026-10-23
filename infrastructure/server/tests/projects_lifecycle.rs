//! Project deletion lifecycle + data exports — service-level flows vs. the
//! auth-store fake and a real-on-disk file config store:
//! creation seeds the project folder (and rolls back on failure), deletion
//! freezes instantly while staying downloadable + cancellable, the sweep
//! purge wipes bucket-folder-first then rows, and the export zip carries
//! the rows plus project files at their real paths with the key secret out.

mod common;

use std::io::Read;
use std::sync::Arc;

use common::{
    FakeApiKeysStore, FakeAuditStore, FakeAuthStore, FakeFlowStore, FakeRecipientsStore,
    FakeSendStore, FakeTemplatesStore, FakeWebhookStore,
};
use server::domain::auth::entities::UserId;
use server::domain::error::AppError;
use server::domain::projects::{PURGE_GRACE, ProjectExportService, ProjectService};
use server::infra::FileConfigStore;
use server::ports::ProjectsStore;
use server::ports::config_store::ConfigStore;

fn audit() -> Arc<server::domain::audit::AuditService> {
    Arc::new(server::domain::audit::AuditService::new(Arc::new(
        FakeAuditStore::new(),
    )))
}

fn temp_root(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("notifi-lifecycle-{name}"));
    std::fs::remove_dir_all(&root).ok();
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn project_service(
    store: Arc<FakeAuthStore>,
    config_store: Option<Arc<FileConfigStore>>,
    audit: Arc<server::domain::audit::AuditService>,
) -> ProjectService {
    let base = ProjectService::new(store as Arc<dyn server::ports::ProjectsStore>, audit);
    match config_store {
        Some(cs) => base.with_config_store(
            cs as Arc<dyn server::ports::config_store::ConfigStore + Send + Sync>,
        ),
        None => base,
    }
}

async fn export_service(
    store: Arc<FakeAuthStore>,
    config_store: Option<Arc<FileConfigStore>>,
) -> ProjectExportService {
    let base = ProjectExportService::new(
        store.clone() as Arc<dyn server::ports::ProjectsStore>,
        Arc::new(FakeTemplatesStore::new())
            as Arc<dyn server::ports::templates_store::TemplatesStore>,
        Arc::new(FakeApiKeysStore::new()) as Arc<dyn server::ports::api_keys_store::ApiKeysStore>,
        Arc::new(FakeSendStore::new()) as Arc<dyn server::ports::send_store::SendStore>,
        Arc::new(FakeRecipientsStore::new())
            as Arc<dyn server::ports::recipients_store::RecipientsStore>,
        Arc::new(FakeWebhookStore::new(Arc::new(FakeFlowStore::new())))
            as Arc<dyn server::ports::webhook_store::WebhookStore>,
        Arc::new(FakeFlowStore::new()) as Arc<dyn server::ports::flow_store::FlowStore>,
    );
    match config_store {
        Some(cs) => base.with_config_store(
            cs as Arc<dyn server::ports::config_store::ConfigStore + Send + Sync>,
        ),
        None => base,
    }
}

// -- creation -------------------------------------------------------------

#[tokio::test]
async fn create_materializes_the_project_folder() {
    let store = Arc::new(FakeAuthStore::new());
    let root = temp_root("materialize");
    let config_store = Arc::new(FileConfigStore::new(&root));
    let service = project_service(store.clone(), Some(config_store.clone()), audit());

    let project = service
        .create(UserId::new(), "Acme Team", None)
        .await
        .expect("create succeeds with materialization");

    // The slugified folder exists with the code-owned default layout,
    // keyed by the project's stored slug (the bucket convention).
    let keys = config_store
        .list_keys(&format!("projects/{}/", project.slug))
        .await
        .expect("project folder listed");
    assert!(!keys.is_empty(), "default files must be present");

    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn failed_materialization_rolls_the_row_back() {
    let store = Arc::new(FakeAuthStore::new());
    // Root under a PLAIN FILE: directory creation inside it always fails.
    let blocker = temp_root("blocker");
    let blocker_file = blocker.join("plain-file");
    std::fs::write(&blocker_file, b"x").unwrap();
    let config_store = Arc::new(FileConfigStore::new(blocker_file.join("child")));
    let service = project_service(store.clone(), Some(config_store), audit());

    let err = service
        .create(UserId::new(), "Doomed Project", None)
        .await
        .expect_err("materialization failure must fail creation");
    assert!(matches!(err, AppError::Storage(_)), "got: {err:?}");

    // Best-effort rollback: no half-created row left behind.
    assert!(
        store
            .list_projects(UserId::new())
            .await
            .expect("list")
            .is_empty(),
        "the failed creation must not leave a row"
    );

    std::fs::remove_dir_all(&blocker).ok();
}

// -- deletion window ------------------------------------------------------

#[tokio::test]
async fn deletion_freezes_then_cancel_restores_everything() {
    let store = Arc::new(FakeAuthStore::new());
    let config_store = Arc::new(FileConfigStore::new(temp_root("freeze")));
    let service = project_service(store.clone(), Some(config_store), audit());

    let owner = UserId::new();
    let project = service
        .create(owner, "Freeze Me", None)
        .await
        .expect("created");

    let tomb = service
        .request_deletion(owner, &project.id)
        .await
        .expect("deletion requested");
    assert!(tomb.deleted_at.is_some(), "tombstone set");
    let gap = tomb.purge_after.unwrap() - tomb.deleted_at.unwrap();
    assert!(
        (gap - PURGE_GRACE) < chrono::Duration::seconds(5),
        "purge deadline = request time + grace (gap was {gap})"
    );

    // Frozen: the access lookup every delivery path uses reads as missing.
    let access = store
        .get_project_access(owner, &project.id)
        .await
        .expect("access query");
    assert!(access.is_none(), "frozen project is invisible to traffic");

    // But members still see it, deadline riding along for the countdown.
    let visible = store.list_projects(owner).await.unwrap();
    let listed = visible
        .iter()
        .find(|p| p.id == project.id)
        .expect("tombstone stays visible to members");
    assert!(listed.purge_after.is_some(), "deadline shown to members");

    let restored = service
        .cancel_deletion(owner, &project.id)
        .await
        .expect("cancel works inside the window");
    assert!(restored.deleted_at.is_none(), "tombstone lifted");
    assert!(restored.purge_after.is_none(), "no deadline left");

    // Nothing was destroyed — access works again.
    let access = store
        .get_project_access(owner, &project.id)
        .await
        .expect("access query");
    assert!(access.is_some(), "project fully restored");
}

#[tokio::test]
async fn deletion_needs_owner_or_admin() {
    let store = Arc::new(FakeAuthStore::new());
    let service = project_service(store.clone(), None, audit());

    let owner = UserId::new();
    let viewer = UserId::new();
    let admin = UserId::new();
    let project = service
        .create(owner, "Roles Test", None)
        .await
        .expect("created");
    store.seed_member(&project.id, &viewer.to_string(), "viewer");
    store.seed_member(&project.id, &admin.to_string(), "admin");

    let err = service
        .request_deletion(viewer, &project.id)
        .await
        .expect_err("viewer must not request deletion");
    assert!(matches!(err, AppError::NotFound(_)), "got: {err:?}");
    assert!(
        store.list_projects(owner).await.unwrap()[0]
            .deleted_at
            .is_none(),
        "state unchanged after the refusal"
    );

    service
        .request_deletion(admin, &project.id)
        .await
        .expect("admin can request deletion");
}

// -- purge ----------------------------------------------------------------

#[tokio::test]
async fn purge_wipes_bucket_folder_first_then_rows() {
    let store = Arc::new(FakeAuthStore::new());
    let root = temp_root("purge");
    let config_store = Arc::new(FileConfigStore::new(&root));
    let service = project_service(store.clone(), Some(config_store.clone()), audit());

    let owner = UserId::new();
    let project = service.create(owner, "Purge Target", None).await.unwrap();
    let prefix = format!("projects/{}/", project.slug);
    config_store
        .put_raw(&format!("{prefix}config/email/config.json"), b"{}".to_vec())
        .await
        .unwrap();
    config_store
        .put_raw(
            &format!("{prefix}templates/welcome/welcome-email.html"),
            b"<p>hi</p>".to_vec(),
        )
        .await
        .unwrap();
    assert!(
        !config_store.list_keys(&prefix).await.unwrap().is_empty(),
        "files staged for the purge"
    );

    // Window runs: nothing purged yet, folder intact.
    let purged = service.purge_due(chrono::Utc::now()).await.unwrap();
    assert_eq!(purged, 0, "no project expires while the window runs");
    assert!(!config_store.list_keys(&prefix).await.unwrap().is_empty());

    // Countdown over: the folder goes first, then the row (cascades own
    // the children).
    store.mark_purge_due(&project.id);
    let purged = service.purge_due(chrono::Utc::now()).await.unwrap();
    assert_eq!(purged, 1, "exactly the one expired project");
    assert!(
        config_store.list_keys(&prefix).await.unwrap().is_empty(),
        "bucket folder erased"
    );
    assert!(
        store
            .list_projects(owner)
            .await
            .unwrap()
            .iter()
            .all(|p| p.id != project.id),
        "row (and cascades) erased"
    );

    // Idempotence: re-running finds nothing due and breaks nothing.
    let again = service.purge_due(chrono::Utc::now()).await.unwrap();
    assert_eq!(again, 0, "re-runs are harmless");

    std::fs::remove_dir_all(&root).ok();
}

// -- exports --------------------------------------------------------------

#[tokio::test]
async fn export_zip_carries_sections_and_project_files() {
    let store = Arc::new(FakeAuthStore::new());
    let root = temp_root("export");
    let config_store = Arc::new(FileConfigStore::new(&root));
    let projects = project_service(store.clone(), Some(config_store.clone()), audit());

    let owner = UserId::new();
    let project = projects.create(owner, "Exportable", None).await.unwrap();
    let member = UserId::new();
    store.seed_member(&project.id, &member.to_string(), "developer");

    // A project file at a real path joins the zip byte-for-byte.
    let project_key = format!("projects/{}/markets/summer-banner.png", project.slug);
    config_store
        .put_raw(&project_key, b"PNGDATA".to_vec())
        .await
        .unwrap();

    // The export works for ANY member (not just the owner).
    let svc = export_service(store.clone(), Some(config_store)).await;
    let bundle = svc.build_zip(member, &project.id).await.expect("zip built");
    assert!(bundle.filename.starts_with("exportable-export-"));

    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&bundle.bytes)).expect("readable zip");
    let names: Vec<String> = zip.file_names().map(str::to_owned).collect::<Vec<_>>();
    for expected in [
        "manifest.json",
        "members.json",
        "templates.json",
        "apiKeys.json",
        "sends.json",
        "recipients.json",
        "webhooks.json",
        "flows.json",
        project_key.as_str(),
    ] {
        assert!(names.iter().any(|n| n == expected), "missing {expected}");
    }

    let manifest: serde_json::Value = {
        let mut manifest_file = zip.by_name("manifest.json").unwrap();
        let mut manifest_bytes = Vec::new();
        manifest_file.read_to_end(&mut manifest_bytes).unwrap();
        serde_json::from_slice(&manifest_bytes).unwrap()
    };
    assert_eq!(manifest["project"]["id"], project.id);
    assert_eq!(manifest["project"]["slug"], project.slug);
    // The projectFiles section counts the project's whole folder (the creation
    // materialized the default layout, plus the banner staged above).
    let counted = manifest["sections"]["projectFiles"]["count"]
        .as_u64()
        .unwrap();
    assert!(counted >= 1, "at least the staged file");
    let paths = manifest["sections"]["projectFiles"]["paths"]
        .as_array()
        .unwrap();
    assert!(paths.iter().any(|p| p == &project_key), "banner listed");

    // Key secrets never export: the section is empty here AND the mapper
    // simply has no field for `key_hash`.
    let api_keys: serde_json::Value = {
        let mut keys_file = zip.by_name("apiKeys.json").unwrap();
        let mut keys_bytes = Vec::new();
        keys_file.read_to_end(&mut keys_bytes).unwrap();
        serde_json::from_slice(&keys_bytes).unwrap()
    };
    assert_eq!(api_keys, serde_json::json!([]));

    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn export_rate_limits_to_a_few_downloads_per_day() {
    let store = Arc::new(FakeAuthStore::new());
    let config_store = Arc::new(FileConfigStore::new(temp_root("rate")));
    let service = project_service(store.clone(), Some(config_store), audit());

    let owner = UserId::new();
    let project = service.create(owner, "Rate Limited", None).await.unwrap();

    let limit = server::domain::projects::EXPORTS_PER_DAY;
    for n in 0..limit {
        service
            .guard_export(owner, &project.id)
            .await
            .unwrap_or_else(|e| panic!("download {n} must pass: {e:?}"));
    }

    let err = service
        .guard_export(owner, &project.id)
        .await
        .expect_err("one download over the line must be refused");
    assert!(matches!(err, AppError::RateLimited(_)), "got: {err:?}");

    // The window is per PROJECT — another project's quota starts fresh.
    let second = service.create(owner, "Quota B", None).await.unwrap();
    service
        .guard_export(owner, &second.id)
        .await
        .expect("separate project, fresh quota");
}

#[tokio::test]
async fn export_of_an_unknown_project_reads_as_missing() {
    let store = Arc::new(FakeAuthStore::new());
    let _service = project_service(store.clone(), None, audit());
    let svc = export_service(store, None).await;

    let err = svc
        .build_zip(UserId::new(), "01HZXNOTREAL")
        .await
        .expect_err("non-member / unknown project");
    assert!(matches!(err, AppError::NotFound(_)), "got: {err:?}");
}
