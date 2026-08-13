//! `cleanup-checkpoints` — Rust port of `src/commands/cleanup-checkpoints.ts`
//! (RPC-203).
//!
//! Deletes the oldest checkpoints for a work unit while preserving the most
//! recent `keepLast` by creation timestamp. Enumerates checkpoint refs via
//! [`codelet_git::ghost_commit::list_ghost_checkpoints`] (pure gitoxide — no
//! `git` CLI), correlates each with the metadata index at
//! `.git/fspec-checkpoints-index/<work_unit_id>.json` to recover timestamps,
//! sorts newest-first, then deletes everything beyond the keep window via
//! [`codelet_git::ghost_commit::delete_ghost_checkpoint`].
//!
//! Parity notes (predecessor findings):
//!   - keepLast validation (`< 1` → error containing
//!     `"--keep-last must be a positive number"`) mirrors the TS
//!     `cleanupCheckpointsCommand` guard (`src/commands/cleanup-checkpoints.ts:85-89`).
//!   - The index is NOT pruned after deletion. The TS
//!     `cleanupCheckpoints` util (`src/utils/git-checkpoint.ts:382-417`) only
//!     deletes refs; it leaves the index file untouched (unlike
//!     `cleanupAutoCheckpoints`). We preserve that exact behaviour.
//!   - `sendIPCMessage` is a documented NO-OP in the Rust standalone binary.
//!
//! Two-front-doors (RPC-003 §7/§11): invoked by BOTH the dispatcher and the
//! `fspec cleanup-checkpoints` clap subcommand — no list/sort/delete/render
//! logic is duplicated in the CLI bridge.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::FspecCoreError;

use codelet_git::ghost_commit::{delete_ghost_checkpoint, list_ghost_checkpoints};

/// CLI arguments accepted by `cleanup-checkpoints`.
///
/// Parity with TS Commander.js registration
/// (`src/commands/cleanup-checkpoints.ts:108-114`): one positional
/// `<work-unit-id>` and a required `--keep-last <number>` option. We expose
/// `format` for the structured dispatcher path.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct CleanupArgs {
    #[serde(default)]
    work_unit_id: Option<String>,
    #[serde(default)]
    keep_last: Option<i64>,
    #[serde(default)]
    format: Option<String>,
}

/// A single checkpoint's displayable metadata (name + recovered timestamp).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckpointEntry {
    name: String,
    timestamp: String,
}

/// Structured cleanup payload. `#[derive(Serialize)]` preserves declaration
/// order, emitting keys as
/// `workUnitId, deletedCount, preservedCount, deleted, preserved`.
///
/// `keep_last` is carried for rendering the header line — TS prints the
/// user-supplied `keepLast` value (`src/commands/cleanup-checkpoints.ts:33`),
/// which can exceed the total checkpoint count. It is `#[serde(skip)]` so the
/// JSON payload keeps the exact five-key shape the dispatcher test pins.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CleanupResult {
    work_unit_id: String,
    deleted_count: usize,
    preserved_count: usize,
    deleted: Vec<CheckpointEntry>,
    preserved: Vec<CheckpointEntry>,
    #[serde(skip)]
    keep_last: usize,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: CleanupArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "cleanup-checkpoints",
            reason: format!("failed to parse args: {e}"),
        })?;

    let work_unit_id = match args.work_unit_id.as_deref() {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => {
            return Err(FspecCoreError::InvalidArgs {
                command: "cleanup-checkpoints",
                reason: "missing or empty `workUnitId` field".to_string(),
            });
        }
    };

    // keepLast must be a positive integer. Mirror the TS guard's EXACT message
    // substring so the CLI bridge can surface it byte-identically.
    let keep_last = match args.keep_last {
        Some(n) if n >= 1 => n as usize,
        _ => {
            return Err(FspecCoreError::InvalidArgs {
                command: "cleanup-checkpoints",
                reason: "--keep-last must be a positive number".to_string(),
            });
        }
    };

    // Enumerate checkpoint refs (pure gitoxide). Unlike the other read paths,
    // a repository-open failure here is PROPAGATED (not swallowed): the TS
    // `listGhostCheckpoints` NAPI binding throws when the directory is not a
    // git repo, and `cleanupCheckpointsCommand` surfaces it as an error
    // (`src/commands/cleanup-checkpoints.ts:73-77`). We mirror that by
    // returning a `Message` error carrying the codelet-git error text verbatim
    // so the CLI bridge renders the same `Failed to open repository ...`
    // string the TS NAPI layer produces.
    let names: Vec<String> = list_ghost_checkpoints(project_root, &work_unit_id)
        .map_err(|e| FspecCoreError::Message(e.to_string()))?;

    // Recover timestamps from the metadata index (ENOENT / malformed → none).
    let index = read_index(project_root, &work_unit_id);

    let mut entries: Vec<CheckpointEntry> = names
        .into_iter()
        .map(|name| {
            let timestamp = lookup_timestamp(&index, &name).unwrap_or_else(now_iso8601);
            CheckpointEntry { name, timestamp }
        })
        .collect();

    // Sort newest-first by timestamp (ISO-8601 strings sort chronologically).
    // Mirrors `src/utils/git-checkpoint.ts:394-397`.
    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    let preserved: Vec<CheckpointEntry> = entries.iter().take(keep_last).cloned().collect();
    let deleted: Vec<CheckpointEntry> = entries.iter().skip(keep_last).cloned().collect();

    // Delete the old checkpoints. Per TS, deletion failures are swallowed and
    // iteration continues (`src/utils/git-checkpoint.ts:403-409`).
    for cp in &deleted {
        let _ = delete_ghost_checkpoint(project_root, &work_unit_id, &cp.name);
    }

    // NOTE: the index file is intentionally NOT pruned here — parity with the
    // TS `cleanupCheckpoints` util which leaves the index untouched.

    // IPC notification is a NO-OP in the standalone binary.

    let result = CleanupResult {
        work_unit_id,
        deleted_count: deleted.len(),
        preserved_count: preserved.len(),
        deleted,
        preserved,
        keep_last,
    };

    match args.format.as_deref() {
        Some("json") => render_json(&result),
        _ => Ok(render_text(&result)),
    }
}

/// Read `.git/fspec-checkpoints-index/<work_unit_id>.json` (ENOENT / malformed
/// JSON → `None`).
fn read_index(project_root: &Path, work_unit_id: &str) -> Option<Value> {
    let path = project_root
        .join(".git")
        .join("fspec-checkpoints-index")
        .join(format!("{work_unit_id}.json"));
    let raw = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str::<Value>(&raw).ok()
}

/// Look up a checkpoint's timestamp in the metadata index by name.
fn lookup_timestamp(index: &Option<Value>, name: &str) -> Option<String> {
    index
        .as_ref()
        .and_then(|v| v.get("checkpoints"))
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|cp| cp.get("name").and_then(|n| n.as_str()) == Some(name))
        })
        .and_then(|cp| cp.get("timestamp").and_then(|t| t.as_str()))
        .map(String::from)
}

/// ISO-8601 UTC timestamp fallback (mirrors `new Date().toISOString()`).
fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs_total = dur.as_secs() as i64;
    let millis = dur.subsec_millis();

    let days = secs_total.div_euclid(86_400);
    let secs_of_day = secs_total.rem_euclid(86_400);
    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;

    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

/// Render the cleanup summary text. Mirrors
/// `src/commands/cleanup-checkpoints.ts:32-54`.
fn render_text(result: &CleanupResult) -> String {
    let mut out = String::new();
    // `\nCleaning up checkpoints for ${workUnitId} (keeping last ${keepLast})...\n`
    out.push('\n');
    out.push_str(&format!(
        "Cleaning up checkpoints for {} (keeping last {})...\n",
        result.work_unit_id, result.keep_last
    ));
    out.push('\n');

    if result.deleted_count > 0 {
        out.push_str(&format!(
            "Deleted {} checkpoint(s):\n",
            result.deleted_count
        ));
        for cp in &result.deleted {
            out.push_str(&format!("  - {} ({})\n", cp.name, cp.timestamp));
        }
        out.push('\n');
    }

    if result.preserved_count > 0 {
        out.push_str(&format!(
            "Preserved {} checkpoint(s):\n",
            result.preserved_count
        ));
        for cp in &result.preserved {
            out.push_str(&format!("  - {} ({})\n", cp.name, cp.timestamp));
        }
        out.push('\n');
    }

    out.push_str(&format!(
        "\u{2713} Cleanup complete: {} deleted, {} preserved",
        result.deleted_count, result.preserved_count
    ));
    out
}

/// Render the structured 2-space-indented JSON payload.
fn render_json(result: &CleanupResult) -> Result<String, FspecCoreError> {
    serde_json::to_string_pretty(result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "cleanup-checkpoints",
        reason: format!("failed to serialize result: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    fn entry(name: &str, ts: &str) -> CheckpointEntry {
        CheckpointEntry {
            name: name.into(),
            timestamp: ts.into(),
        }
    }

    #[test]
    fn render_text_contains_header_and_banner() {
        let result = CleanupResult {
            work_unit_id: "AUTH-001".into(),
            deleted_count: 2,
            preserved_count: 1,
            deleted: vec![
                entry("cp-00", "2026-06-01T00:00:00.000Z"),
                entry("cp-01", "2026-06-01T00:01:00.000Z"),
            ],
            preserved: vec![entry("cp-02", "2026-06-01T00:02:00.000Z")],
            keep_last: 1,
        };
        let out = render_text(&result);
        assert!(out.contains("Cleaning up checkpoints for AUTH-001 (keeping last 1)"));
        assert!(out.contains("Deleted 2 checkpoint(s):"));
        assert!(out.contains("Preserved 1 checkpoint(s):"));
        assert!(out.contains("\u{2713} Cleanup complete: 2 deleted, 1 preserved"));
    }

    #[test]
    fn render_text_header_uses_keep_last_not_preserved_count() {
        // When keepLast exceeds the total checkpoint count, the header must
        // echo the user-supplied keepLast (parity with TS), NOT the smaller
        // preserved count.
        let result = CleanupResult {
            work_unit_id: "AUTH-001".into(),
            deleted_count: 0,
            preserved_count: 2,
            deleted: vec![],
            preserved: vec![
                entry("a", "2026-06-01T00:00:00.000Z"),
                entry("b", "2026-06-01T00:01:00.000Z"),
            ],
            keep_last: 100,
        };
        let out = render_text(&result);
        assert!(out.contains("Cleaning up checkpoints for AUTH-001 (keeping last 100)"));
    }

    #[test]
    fn json_key_order() {
        let result = CleanupResult {
            work_unit_id: "AUTH-001".into(),
            deleted_count: 0,
            preserved_count: 0,
            deleted: vec![],
            preserved: vec![],
            keep_last: 5,
        };
        let data: Value = serde_json::from_str(&render_json(&result).unwrap()).unwrap();
        let keys: Vec<&str> = data
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            vec![
                "workUnitId",
                "deletedCount",
                "preservedCount",
                "deleted",
                "preserved"
            ]
        );
    }
}
