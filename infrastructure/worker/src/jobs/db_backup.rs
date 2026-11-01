//! Database backups — the self-contained ops block in the worker.
//!
//! Once a day the worker exports the Postgres database as ONE plain-text
//! SQL file (`pg_dump`), uploads it into the SAME object-storage bucket
//! the config store already uses — under its own top-level `backup/`
//! folder — and deletes the local temp file. That file is a full logical
//! backup: every table and every row as SQL, restorable into an empty
//! database with `psql -f`. No second database, no replication — just a
//! file in a bucket, inside a folder nothing else writes to.
//!
//! Deliberate boundaries:
//! * Never the queue or outbox. A queued job needs something to enqueue
//!   it forever; a clock is simpler and honest — same philosophy as the
//!   sweep's local interval.
//! * Isolation is key-prefix discipline: this block only ever reads and
//!   writes keys under `backup/`, and every destructive flow in the
//!   codebase (config reset, future project deletion) only operates on
//!   its own prefixes, never `backup/`. The list probe here looks at
//!   `backup/` alone. A separate bucket is the harder-isolation upgrade;
//!   switching to it later is one env change.
//! * Nothing in the backup folder is EVER deleted automatically. Files
//!   only accumulate; retention is a human decision, made later.
//! * When the backup runs: daily at `NOTIFI_BACKUP_AT` (default 03:00,
//!   server-local). At boot it dumps immediately if today's backup is
//!   missing, so restarts never lose a day. Two workers starting at the
//!   exact same second race into two same-day files (different
//!   timestamps) — a prod footnote, not engineered away here.
//!
//! Off unless explicitly switched on with `NOTIFI_BACKUP_ENABLED=true`.
//! R2 access rides the same `NOTIFI_R2_*` quartet the whole app uses —
//! no separate credentials, no separate bucket. Dump binary:
//! `NOTIFI_BACKUP_PG_DUMP` (default `pg_dump`); the host needs the
//! Postgres client tools.
//!
//! One-shot, no waiting for the slot (works whether or not the switch
//! is on — explicit intent wins):
//! `cargo run -p worker -- --backup-now`
//!
//! Upload shape: the temp file streams into a multipart upload in fixed
//! 8 MiB parts (R2 requires ≥5 MiB equal-sized parts). Machine memory
//! never holds the whole dump, so gigabyte databases need no rewrite.

use std::io::Read;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Datelike, Days, Local};
use futures::StreamExt;
use object_store::path::Path as ObjectPath;
use object_store::{ObjectStore, ObjectStoreExt, PutPayload};

use server::infra::config::AppConfig;
use server::infra::r2_config_store::raw_object_client;

/// Fixed part size: R2 wants all-but-last parts ≥5 MiB and equal-sized,
/// so one constant satisfies both and files up to 80 GiB stay legal.
const CHUNK_SIZE: u64 = 8 * 1024 * 1024;

/// Top-level folder every backup object shares inside the shared bucket
/// (flat layout within it: dates in the filename, so a prefix list sees
/// the whole history at once). Nothing else in the codebase writes here.
const KEY_PREFIX: &str = "backup";

pub(crate) struct BackupSettings {
    client: Arc<dyn ObjectStore>,
    /// Bucket surfaced in logs and error messages only.
    bucket: String,
    /// Daily slot in server-local time, hour + minute (0-23 / 0-59).
    at_hour: u32,
    at_minute: u32,
    /// Path to the `pg_dump` binary (overridable for custom installs).
    pg_dump_path: String,
    /// Full connection URL, password included.
    db_url: String,
}

impl BackupSettings {
    /// Parses the environment into settings. Returns `None` when backups
    /// are off (the switch is not on); hard errors on R2 creds / db URL
    /// are logged and still leave the worker backup-less rather than
    /// broken. Storage is the same R2 setting everything else reads
    /// (`NOTIFI_R2_*` quartet, the shared bucket) — the `backup/` folder
    /// of that bucket is where this module's keys live.
    /// Scheduler entry: respects the `NOTIFI_BACKUP_ENABLED` switch.
    pub(crate) fn from_env(app: &AppConfig) -> Option<Self> {
        if !backup_switch_on() {
            return None;
        }
        Self::build(app)
    }

    /// `--backup-now` entry: explicit user intent wins over the switch —
    /// this runs whether or not `NOTIFI_BACKUP_ENABLED` is on.
    pub(crate) fn for_one_shot(app: &AppConfig) -> Option<Self> {
        Self::build(app)
    }

    /// The only difference between the two entries is the switch gate —
    /// storage, slot, binary and URL parsing are identical.
    fn build(app: &AppConfig) -> Option<Self> {
        let (client, bucket) = match raw_object_client(&app.r2) {
            Ok(pair) => pair,
            Err(e) => {
                tracing::error!("database backups disabled ({e})");
                return None;
            }
        };

        let at = std::env::var("NOTIFI_BACKUP_AT")
            .ok()
            .unwrap_or_else(|| "03:00".to_string());
        let (at_hour, at_minute) = match parse_at_time(&at) {
            Some(pair) => pair,
            None => {
                tracing::error!(
                    provided = %at,
                    "NOTIFI_BACKUP_AT is not HH:MM (24h); falling back to 03:00"
                );
                (3, 0)
            }
        };

        let pg_dump_path = std::env::var("NOTIFI_BACKUP_PG_DUMP")
            .ok()
            .filter(|p| !p.trim().is_empty())
            .unwrap_or_else(|| "pg_dump".to_string());

        let db_url = std::env::var("NOTIFI_BACKUP_DB_URL")
            .ok()
            .filter(|u| !u.trim().is_empty())
            .or_else(|| std::env::var("NOTIFI_DATABASE_URL").ok())
            .filter(|u| !u.trim().is_empty());
        let Some(db_url) = db_url else {
            tracing::error!(
                "database backups disabled: no connection URL \
                 (NOTIFI_BACKUP_DB_URL or NOTIFI_DATABASE_URL)"
            );
            return None;
        };

        Some(Self {
            client,
            bucket,
            at_hour,
            at_minute,
            pg_dump_path,
            db_url,
        })
    }

    /// True once a backup exists whose key carries today's date stamp.
    /// A listing error means "unknown" — treated as not backed up, so a
    /// transient failure repeats a dump rather than losing a day.
    async fn backed_up_today(&self) -> bool {
        let now = Local::now();
        let today = format!("{:04}{:02}{:02}", now.year(), now.month(), now.day());
        let mut stream = self.client.list(Some(&ObjectPath::from(KEY_PREFIX)));
        while let Some(meta) = stream.next().await {
            match meta {
                Ok(meta) => {
                    if meta.location.as_ref().contains(&today) {
                        return true;
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, bucket = %self.bucket, "backup bucket listing failed; assuming today is not backed up");
                    return false;
                }
            }
        }
        false
    }

    /// Dumps the whole database to a temp file, streams it into the
    /// bucket, deletes the temp file. The returned key is the uploaded
    /// object's name.
    pub(crate) async fn run_once(&self) -> Result<String, String> {
        let start = Instant::now();
        let dbname = database_name(&self.db_url);
        let stamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
        let key = format!("{KEY_PREFIX}/postgres-{dbname}-{stamp}.sql");
        let tmp = std::env::temp_dir().join(format!("notifi-backup-{stamp}.sql"));

        // pg_dump runs blocking in its own thread so the async runtime
        // keeps sweeping while the dump proceeds. `--format=plain` gives
        // the text-SQL file the feature is about; `--no-owner
        // --no-privileges` strip ALTER OWNER/GRANT lines so the dump
        // restores into any fresh database cleanly.
        let dump_bin = self.pg_dump_path.clone();
        let dump_bin_for_err = dump_bin.clone();
        let db_url = self.db_url.clone();
        let tmp_path = tmp.clone();
        let output = tokio::task::spawn_blocking(move || {
            std::process::Command::new(&dump_bin)
                .arg("--format=plain")
                .arg("--no-owner")
                .arg("--no-privileges")
                .arg("--file")
                .arg(&tmp_path)
                .arg("--dbname")
                .arg(&db_url)
                .output()
        })
        .await
        .map_err(|e| format!("backup dump task join failed: {e}"))?
        .map_err(|e| format!("backup failed to run '{dump_bin_for_err}': {e}"))?;

        if !output.status.success() {
            std::fs::remove_file(&tmp).ok();
            return Err(format!(
                "pg_dump failed ({}): {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }

        let file_len = std::fs::metadata(&tmp)
            .map_err(|e| format!("backup temp file missing after dump: {e}"))?
            .len();

        let path =
            ObjectPath::parse(&key).map_err(|e| format!("invalid backup key '{key}': {e}"))?;
        let upload_result: Result<(), String> = self.stream_file(&tmp, file_len, &path).await;

        // The temp file dies either way — success or not.
        std::fs::remove_file(&tmp).ok();

        upload_result?;
        tracing::info!(
            key = %key,
            size_bytes = file_len,
            bucket = %self.bucket,
            elapsed_ms = start.elapsed().as_millis() as u64,
            "database backup uploaded"
        );
        Ok(key)
    }

    /// Streams a local file into the bucket in fixed [`CHUNK_SIZE`]
    /// parts. A zero-length file uploads as a single empty put instead
    /// (participles-of-empty edge, essentially impossible for a dump).
    async fn stream_file(
        &self,
        tmp: &std::path::Path,
        file_len: u64,
        path: &ObjectPath,
    ) -> Result<(), String> {
        if file_len == 0 {
            self.client
                .put(path, PutPayload::default())
                .await
                .map_err(|e| format!("backup upload failed (bucket {}): {e}", self.bucket))?;
            return Ok(());
        }

        let mut upload = self
            .client
            .put_multipart(path)
            .await
            .map_err(|e| format!("backup upload start failed (bucket {}): {e}", self.bucket))?;

        let result: Result<u64, String> = async {
            let mut file = std::fs::File::open(tmp)
                .map_err(|e| format!("backup temp file unopenable: {e}"))?;
            let mut remaining = file_len;
            while remaining > 0 {
                let take = CHUNK_SIZE.min(remaining);
                let mut buf = vec![0u8; take as usize];
                // Blocking disk read of ≤8 MiB, once per day; fine as
                // async hygiene trade-off for ops code.
                file.read_exact(&mut buf)
                    .map_err(|e| format!("backup temp read failed: {e}"))?;
                upload
                    .put_part(PutPayload::from(buf))
                    .await
                    .map_err(|e| format!("backup upload part failed: {e}"))?;
                remaining -= take;
            }
            upload
                .complete()
                .await
                .map_err(|e| format!("backup upload finalize failed: {e}"))?;
            Ok(file_len)
        }
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                // Best-effort cleanup of the half-uploaded object; the
                // bucket keeps nothing partial if R2 cooperates.
                let _ = upload.abort().await;
                Err(e)
            }
        }
    }
}

/// The forever loop: dump now if today is missing, then sleep to the
/// daily slot, forever. Errors log and the next slot retries — the
/// worker never dies from a backup.
pub(crate) async fn schedule(settings: BackupSettings) {
    let at_display = format!("{:02}:{:02}", settings.at_hour, settings.at_minute);
    tracing::info!(
        bucket = %settings.bucket,
        at = %at_display,
        "worker: database backups scheduled"
    );
    loop {
        if !settings.backed_up_today().await
            && let Err(e) = settings.run_once().await
        {
            tracing::error!(error = %e, "database backup failed (retries at the next slot)");
        }
        let now = Local::now();
        let wait = duration_until_next(now, settings.at_hour, settings.at_minute);
        tracing::debug!(wait_secs = wait.as_secs(), "backups: next slot in");
        tokio::time::sleep(wait).await;
    }
}

/// One-shot used by `--backup-now`: builds settings (ignoring the
/// enabled switch — the flag itself is the intent), dumps, uploads,
/// returns the process exit code.
pub(crate) async fn one_shot(app: &AppConfig) -> i32 {
    let Some(settings) = BackupSettings::for_one_shot(app) else {
        eprintln!(
            "--backup-now: storage not configured needs the NOTIFI_R2_* \
             quartet and a database URL (NOTIFI_BACKUP_DB_URL or \
             NOTIFI_DATABASE_URL)"
        );
        return 1;
    };
    match settings.run_once().await {
        Ok(key) => {
            println!("backup uploaded: {key} (bucket {})", settings.bucket);
            0
        }
        Err(e) => {
            eprintln!("--backup-now failed: {e}");
            1
        }
    }
}

/// `NOTIFI_BACKUP_ENABLED` truthy check — accepts true / 1 / yes / on
/// (case-insensitive). Anything else, including absence, is off.
fn backup_switch_on() -> bool {
    let value = std::env::var("NOTIFI_BACKUP_ENABLED")
        .ok()
        .unwrap_or_default();
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "1" | "yes" | "on"
    )
}

/// "HH:MM" (24h) → `(hour, minute)`. Accepts zero-padded or not; rejects
/// anything out of range.
fn parse_at_time(value: &str) -> Option<(u32, u32)> {
    let (h, m) = value.trim().split_once(':')?;
    let h: u32 = h.trim().parse().ok()?;
    let m: u32 = m.trim().parse().ok()?;
    if h <= 23 && m <= 59 {
        Some((h, m))
    } else {
        None
    }
}

/// Duration from `now` to the next daily occurrence of `h:m` server-local.
/// Equal to the slot = already passed today → tomorrow. DST gaps fall
/// back to "now", which then routes to the next day rather than drifting
/// forever.
fn duration_until_next(now: DateTime<Local>, h: u32, m: u32) -> Duration {
    let slot = |date: chrono::NaiveDate| {
        date.and_hms_opt(h, m, 0)
            .expect("h/m validated at parse")
            .and_local_timezone(now.timezone())
            .single()
    };
    let target = match slot(now.date_naive()) {
        Some(t) if t > now => t,
        _ => slot(now.date_naive() + Days::new(1)).unwrap_or(now + chrono::Duration::hours(24)),
    };
    (target - now).to_std().unwrap_or(Duration::from_secs(1))
}

/// Database name for filenames: last path segment of the connection URL
/// before any `?query`, trailing slash trimmed. `unknown` for weird URLs
/// so filenames still form.
fn database_name(url: &str) -> String {
    let no_query = url.split('?').next().unwrap_or("");
    // No trim before the slice: on a URL ending in '/' the final segment
    // is empty and that emptiness IS the answer; trimming first would
    // collapse the path onto the authority (`u:p@h:5432`) and badge the
    // connection string's credentials as the database "name".
    let name = no_query.rsplit('/').next().unwrap_or("").trim().to_string();
    if name.is_empty() {
        "unknown".to_string()
    } else {
        name
    }
}

#[cfg(test)]
mod backup_tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn slot_parsing_accepts_ranges_and_rejects_garbage() {
        assert_eq!(parse_at_time("03:00"), Some((3, 0)));
        assert_eq!(parse_at_time("3:15"), Some((3, 15)));
        assert_eq!(parse_at_time("23:59"), Some((23, 59)));
        assert_eq!(parse_at_time("24:00"), None);
        assert_eq!(parse_at_time("12:60"), None);
        assert_eq!(parse_at_time("abc"), None);
        assert_eq!(parse_at_time(""), None);
    }

    #[test]
    fn database_names_come_from_the_url_path() {
        assert_eq!(
            database_name("postgresql://user:secretpass@db:5432/notifi?sslmode=require"),
            "notifi"
        );
        assert_eq!(database_name("postgres://u:p@h:1/testdb"), "testdb");
        assert_eq!(database_name("postgresql://u:p@h:5432/"), "unknown");
        assert_eq!(database_name(""), "unknown");
    }

    #[test]
    fn next_run_sleeps_to_the_remaining_time() {
        let now_at = |h: u32, m: u32| Local.with_ymd_and_hms(2026, 10, 23, h, m, 0).unwrap();
        // Morning, slot later today → same-day 03:00, one hour out.
        let dur = duration_until_next(now_at(2, 0), 3, 0);
        assert_eq!(dur.as_secs(), 3600);
        // Slot equals now → treated as passed → tomorrow (24h).
        let dur = duration_until_next(now_at(3, 0), 3, 0);
        assert_eq!(dur.as_secs(), 24 * 3600);
        // Late evening → wraps to tomorrow 03:00 (exactly 3h30m out).
        let dur = duration_until_next(now_at(23, 30), 3, 0);
        assert_eq!(dur.as_secs(), 3 * 3600 + 30 * 60);
    }
}
