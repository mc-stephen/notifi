//! `.env.example` ↔ code parity guard.
//!
//! The template is the checked-in inventory of every `NOTIFI_*` variable
//! any consumer reads (boot config, worker settings, provider probe
//! tests, gated suites). CI fails here when the two drift apart, so
//! "what needs to go into the real thing" is always answerable by
//! looking at one file.
//!
//! How: every quoted string in the production sources and their gated
//! tests is scanned for `NOTIFI_...` tokens — that catches both
//! `std::env::var("NOTIFI_…")` reads and the `#[ignore = "needs …"]`
//! attribute messages naming a probe's requirements. The template must
//! contain exactly those names (commented or not), and the uncommented
//! ones must be exactly the documented required set.

use std::collections::BTreeSet;
use std::path::PathBuf;

/// Path to the template from this test's crate (`server`).
fn template_path() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo manifest dir"))
        .parent()
        .expect("server crate has a parent")
        .join(".env.example")
}

/// Source trees whose quoted literals count as real consumers. Missing
/// dirs (e.g. no worker tests) are fine.
fn source_dirs() -> Vec<PathBuf> {
    let root = std::env::var("CARGO_MANIFEST_DIR").expect("cargo manifest dir");
    let server = PathBuf::from(&root);
    let worker = server
        .parent()
        .expect("server crate has a parent")
        .join("worker");
    vec![
        server.join("src"),
        server.join("tests"),
        worker.join("src"),
        worker.join("tests"),
    ]
}

/// Walks `dir` for `.rs` files (missing dirs silently skipped).
fn walk_rs_files(dir: &std::path::Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_rs_files(&path, files);
        } else if path.extension().is_some_and(|e| e == "rs") {
            files.push(path);
        }
    }
}

/// All `NOTIFI_...` env names inside double-quoted string literals of
/// `text` (naive scan: our literals contain no escaped quotes).
fn quoted_notifi_names(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut names = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'"' {
            i += 1;
            continue;
        }
        let closing = text[i + 1..].find('"');
        let Some(end) = closing else { break };
        let literal = &text[i + 1..i + 1 + end];
        take_names(literal, &mut names);
        i = i + 1 + end + 1;
    }
    names
}

/// Extends `names` with each `NOTIFI_…` token in the literal.
fn take_names(literal: &str, names: &mut Vec<String>) {
    let mut rest = literal;
    while let Some(pos) = rest.find("NOTIFI_") {
        let candidate = &rest[pos..];
        let mut end = "NOTIFI_".len();
        while end < candidate.len()
            && (candidate.as_bytes()[end] == b'_'
                || candidate.as_bytes()[end].is_ascii_uppercase()
                || candidate.as_bytes()[end].is_ascii_digit())
        {
            end += 1;
        }
        // A trailing `_` (prose like `NOTIFI_R2_*`) is not a real name —
        // trim, then skip the match so the move forward never loops.
        let mut token = candidate[..end].to_string();
        let starred = candidate[end..].starts_with('*');
        while token.ends_with('_') {
            token.pop();
        }
        if !starred && token.len() > "NOTIFI_".len() {
            names.push(token);
        }
        let forward = if end > "NOTIFI_".len() {
            end
        } else {
            "NOTIFI_".len()
        };
        rest = &candidate[forward..];
    }
}

fn consumed_names() -> BTreeSet<String> {
    let mut files = Vec::new();
    for dir in source_dirs() {
        walk_rs_files(&dir, &mut files);
    }
    let mut names = BTreeSet::new();
    for file in files {
        if let Ok(text) = std::fs::read_to_string(&file) {
            names.extend(quoted_notifi_names(&text));
        }
    }
    names
}

/// One template key + whether it is commented out.
fn parse_template(text: &str) -> Vec<(String, bool)> {
    let mut entries = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        let after_hash = trimmed.strip_prefix('#').map(str::trim_start);
        let (name_start, active) = match after_hash {
            Some(after) => (after, false),
            None => (trimmed, true),
        };
        if !name_start.starts_with("NOTIFI_") {
            continue;
        }
        let end = name_start
            .find(|c: char| !(c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'))
            .unwrap_or(name_start.len());
        // Prose tokens like `NOTIFI_*` (bare prefix, no real suffix) are
        // not entries.
        if end <= "NOTIFI_".len() {
            continue;
        }
        entries.push((name_start[..end].to_string(), active));
    }
    entries
}

/// The exact set the mixed style promises: the one key a deployment
/// genuinely must set — the database, or nothing boots. Everything else
/// stays commented out with its placeholder/example in place (system
/// mail, OAuth, R2… are optional at boot and documented as effectively
/// required for real deployments right above their own sections).
const REQUIRED_OPEN: [&str; 1] = ["NOTIFI_DATABASE_URL"];

#[test]
fn template_lists_every_consumed_name_and_nothing_else() {
    let text =
        std::fs::read_to_string(template_path()).expect(".env.example exists at infrastructure/");
    let entries = parse_template(&text);
    assert!(
        entries.len() >= 60,
        "template must stay a full inventory; only {} entries found",
        entries.len()
    );

    let consumed = consumed_names();
    let listed: BTreeSet<String> = entries.iter().map(|(name, _)| name.clone()).collect();

    let undocumented: Vec<String> = consumed.difference(&listed).cloned().collect();
    assert!(
        undocumented.is_empty(),
        "code reads these NOTIFI_* names but the template does not list them \
         (add each to infrastructure/.env.example in the same change): {undocumented:?}"
    );

    let phantom: Vec<String> = listed.difference(&consumed).cloned().collect();
    assert!(
        phantom.is_empty(),
        "template lists these NOTIFI_* names but no consumer reads them \
         (remove or rename them in infrastructure/.env.example): {phantom:?}"
    );

    // Sanity for the guard itself: the scan must cover every tier
    // (boot config / worker backups / provider probes / gated suites)
    // or it went blind without failing loudly enough.
    assert!(
        consumed.contains("NOTIFI_DATABASE_URL")
            && consumed.contains("NOTIFI_BACKUP_AT")
            && consumed.contains("NOTIFI_TERMII_API_KEY")
            && consumed.contains("NOTIFI_TEST_REDIS_URL"),
        "the source scan lost tiers — the guard no longer sees all consumers"
    );
}

#[test]
fn only_required_keys_are_uncommented() {
    let text = std::fs::read_to_string(template_path()).expect("template exists");
    let entries = parse_template(&text);

    let open: BTreeSet<&String> = entries
        .iter()
        .filter(|(_, active)| *active)
        .map(|(name, _)| name)
        .collect();
    let expected: BTreeSet<&str> = REQUIRED_OPEN.iter().copied().collect();
    let opened: BTreeSet<&str> = open.iter().map(|s| s.as_str()).collect();
    assert_eq!(
        opened, expected,
        "uncommented keys drifted from the required set; optional keys must stay commented out"
    );

    // Placeholder discipline: no uncommented key may carry a value that
    // looks like a real secret.
    for (name, _) in entries.iter().filter(|(_, active)| *active) {
        let raw_line = text
            .lines()
            .find(|l| l.trim_start().starts_with(name.as_str()))
            .expect("the active key's own line");
        let value = raw_line
            .split_once('=')
            .map(|(_, v)| v.trim())
            .unwrap_or("");
        let placeholder_like = value.contains("change-me")
            || value.contains("<paste")
            || value.contains("your-")
            || value.is_empty();
        assert!(
            placeholder_like,
            "'{name}' carries a value that looks like a real secret; the template \
             must only ever hold placeholders"
        );
    }
}

#[test]
fn every_name_lists_once() {
    let text = std::fs::read_to_string(template_path()).expect("template exists");
    let entries = parse_template(&text);
    let mut counts = std::collections::BTreeMap::new();
    for (name, _) in &entries {
        *counts.entry(name.clone()).or_insert(0u8) += 1;
    }
    let dupes: Vec<String> = counts
        .iter()
        .filter(|(_, c)| **c > 1)
        .map(|(k, _)| k.clone())
        .collect();
    assert!(
        dupes.is_empty(),
        "each name should appear exactly once; duplicates: {dupes:?}"
    );
}
