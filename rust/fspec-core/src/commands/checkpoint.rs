//! `checkpoint` — Rust port of `src/commands/checkpoint.ts` (RPC-202).
//!
//! Creates a manual ghost-commit checkpoint capturing the current working
//! tree state (staged, unstaged, untracked) under
//! `refs/fspec-checkpoints/<work_unit_id>/<checkpoint_name>`, then persists a
//! metadata entry to `.git/fspec-checkpoints-index/<work_unit_id>.json` so the
//! sibling `list-checkpoints`/`cleanup-checkpoints` commands can recover the
//! original creation timestamp.
//!
//! Parity notes (predecessor findings, baked into the spec):
//!   - The capture is delegated to
//!     [`codelet_git::ghost_commit::create_ghost_commit`] — the SAME primitive
//!     the TS NAPI binding (`createGhostCheckpoint`) wraps. codelet-git uses
//!     **pure gitoxide (gix)** — no `git` CLI is ever spawned.
//!   - When the working tree is clean the ghost-commit captures zero changed
//!     files; we mirror the TS early-return: `success:false`, empty
//!     `capturedFiles`, and NO index file is written
//!     (`src/utils/git-checkpoint.ts:166-198`).
//!   - The index write (mkdir-p, read-or-init, dedupe-by-name, append,
//!     pretty-print 2-space) mirrors `updateCheckpointIndex`
//!     (`src/utils/git-checkpoint.ts:71-109`). codelet-git does NOT own the
//!     index; this command owns it.
//!   - `sendIPCMessage({ type: 'checkpoint-changed' })` is a TUI notification
//!     in TS. In the Rust standalone binary there is no IPC channel, so it is
//!     a documented NO-OP.
//!
//! Per RPC-003 §7/§11 (two-front-doors invariant) this single function is
//! invoked by BOTH the LLM-facing dispatcher AND the standalone fspec binary's
//! `fspec checkpoint` clap subcommand — no capture, index-write, or rendering
//! logic is duplicated in the CLI bridge.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::FspecCoreError;

use codelet_git::ghost_commit::create_ghost_commit;

/// CLI arguments accepted by `checkpoint`.
///
/// Parity with TS Commander.js registration
/// (`src/commands/checkpoint.ts:80-89`): two positional arguments
/// `<work-unit-id>` and `<checkpoint-name>`, no `.option(...)` flags. We expose
/// `format` ("text" | "json") for the structured dispatcher path.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct CheckpointArgs {
    #[serde(default)]
    work_unit_id: Option<String>,
    #[serde(default)]
    checkpoint_name: Option<String>,
    #[serde(default)]
    format: Option<String>,
}

/// Structured payload — mirrors the TS return shape at
/// `src/commands/checkpoint.ts:40-46`. `#[derive(Serialize)]` preserves field
/// declaration order so the JSON keys are emitted as
/// `success, checkpointName, capturedFiles, includedUntracked` — the order the
/// test asserts. (TS also returns `stashMessage`/`stashRef`; the Rust port
/// omits those from the structured payload because the dispatcher contract
/// only documents the four keys the test pins and their values are
/// non-deterministic timestamps.)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckpointResult {
    success: bool,
    checkpoint_name: String,
    captured_files: Vec<String>,
    included_untracked: bool,
}

/// Dispatcher entry point. The dispatcher passes the canonical project root
/// alongside the raw JSON args; we never call `std::env::current_dir()`.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: CheckpointArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "checkpoint",
            reason: format!("failed to parse args: {e}"),
        })?;

    let work_unit_id = match args.work_unit_id.as_deref() {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => {
            return Err(FspecCoreError::InvalidArgs {
                command: "checkpoint",
                reason: "missing or empty `workUnitId` field".to_string(),
            });
        }
    };

    let checkpoint_name = match args.checkpoint_name.as_deref() {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => {
            return Err(FspecCoreError::InvalidArgs {
                command: "checkpoint",
                reason: "missing or empty `checkpointName` field".to_string(),
            });
        }
    };

    // Capture the working tree via codelet-git (pure gitoxide). Any error
    // degrades to a failed capture (success:false, empty list) — matching the
    // TS bare-catch at `src/utils/git-checkpoint.ts:188-197`. The returned
    // `files` are the changed files (vs HEAD), i.e. the captured set.
    let (captured_files, sha): (Vec<String>, String) =
        match create_ghost_commit(project_root, &work_unit_id, &checkpoint_name) {
            Ok(ghost) => (ghost.files, ghost.sha),
            Err(_) => (Vec::new(), String::new()),
        };

    // Clean working tree → nothing captured → failure, NO index write.
    // Mirrors `src/utils/git-checkpoint.ts:166-175`.
    if captured_files.is_empty() {
        let result = CheckpointResult {
            success: false,
            checkpoint_name,
            captured_files: Vec::new(),
            included_untracked: true,
        };
        return match args.format.as_deref() {
            Some("json") => render_json(&result),
            _ => Ok(render_text(&work_unit_id, &result)),
        };
    }

    // Persist the metadata index. codelet-git does not own this file; the
    // checkpoint command is its sole writer (predecessor finding).
    write_index(project_root, &work_unit_id, &checkpoint_name, &sha)?;

    // IPC notification is a NO-OP in the Rust standalone binary (no TUI
    // channel). See module docs.

    let result = CheckpointResult {
        success: true,
        checkpoint_name,
        captured_files,
        included_untracked: true,
    };

    match args.format.as_deref() {
        Some("json") => render_json(&result),
        _ => Ok(render_text(&work_unit_id, &result)),
    }
}

/// Append the checkpoint metadata entry to
/// `.git/fspec-checkpoints-index/<work_unit_id>.json`, creating the directory
/// and file as needed. Dedupes by name (parity with TS `updateCheckpointIndex`)
/// and pretty-prints with 2-space indentation.
fn write_index(
    project_root: &Path,
    work_unit_id: &str,
    checkpoint_name: &str,
    sha: &str,
) -> Result<(), FspecCoreError> {
    let index_dir = project_root.join(".git").join("fspec-checkpoints-index");
    std::fs::create_dir_all(&index_dir).map_err(|e| FspecCoreError::Io {
        command: "checkpoint",
        source: e,
    })?;
    let index_path = index_dir.join(format!("{work_unit_id}.json"));

    // Read existing index or start with an empty checkpoints array. Both
    // ENOENT and malformed JSON degrade to an empty index (parity with the TS
    // bare-catch at `src/utils/git-checkpoint.ts:90-95`).
    let mut checkpoints: Vec<Value> = std::fs::read_to_string(&index_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|v| v.get("checkpoints").and_then(|c| c.as_array()).cloned())
        .unwrap_or_default();

    // Add only if not already present (dedupe by name).
    let exists = checkpoints
        .iter()
        .any(|cp| cp.get("name").and_then(|n| n.as_str()) == Some(checkpoint_name));
    if !exists {
        // Mirror TS entry shape `{ name, sha, timestamp }`. The ghost-commit
        // SHA is always non-empty after a successful capture (HEAD existed in
        // every test fixture); fall back to a sentinel only in the unlikely
        // empty-SHA case so the index never carries an empty `sha`.
        let sha_value = if sha.is_empty() { "pending" } else { sha };
        checkpoints.push(json!({
            "name": checkpoint_name,
            "sha": sha_value,
            "timestamp": now_iso8601(),
        }));
    }

    let payload = json!({ "checkpoints": checkpoints });
    let serialized =
        serde_json::to_string_pretty(&payload).map_err(|e| FspecCoreError::InvalidArgs {
            command: "checkpoint",
            reason: format!("failed to serialize index: {e}"),
        })?;
    std::fs::write(&index_path, serialized).map_err(|e| FspecCoreError::Io {
        command: "checkpoint",
        source: e,
    })?;
    Ok(())
}

/// ISO-8601 UTC timestamp (millis, `Z`). Mirrors the TS
/// `new Date().toISOString()` shape. Reuses the same civil-time decomposition
/// as `list_checkpoints::fallback_timestamp` to avoid a chrono dependency.
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

/// Render the human-facing text. Mirrors `src/commands/checkpoint.ts:34-35`:
///   `✓ Created checkpoint "<name>" for <workUnitId>`
///   `  Captured <n> file(s)`
fn render_text(work_unit_id: &str, result: &CheckpointResult) -> String {
    // Parity with `src/commands/checkpoint.ts:34-35`: the TS command always
    // emits the success banner (even on the clean-tree / zero-capture path,
    // where `result.success` is false but the structured return still drives
    // exit code 1). The capture count reflects the actual captured set, so a
    // clean tree renders `Captured 0 file(s)`.
    format!(
        "\u{2713} Created checkpoint \"{}\" for {}\n  Captured {} file(s)",
        result.checkpoint_name,
        work_unit_id,
        result.captured_files.len()
    )
}

/// Render the structured 2-space-indented JSON payload.
fn render_json(result: &CheckpointResult) -> Result<String, FspecCoreError> {
    serde_json::to_string_pretty(result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "checkpoint",
        reason: format!("failed to serialize result: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: CheckpointArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","checkpointName":"baseline"}"#)
                .unwrap();
        assert_eq!(a.work_unit_id.as_deref(), Some("AUTH-001"));
        assert_eq!(a.checkpoint_name.as_deref(), Some("baseline"));
    }

    #[test]
    fn now_iso8601_is_24_chars() {
        let ts = now_iso8601();
        assert_eq!(ts.len(), 24);
        assert!(ts.ends_with('Z'));
    }

    #[test]
    fn render_text_success_banner() {
        let r = CheckpointResult {
            success: true,
            checkpoint_name: "baseline".into(),
            captured_files: vec!["a.txt".into(), "b.txt".into(), "c.txt".into()],
            included_untracked: true,
        };
        let out = render_text("AUTH-001", &r);
        assert!(out.contains("\u{2713} Created checkpoint \"baseline\" for AUTH-001"));
        assert!(out.contains("Captured 3 file(s)"));
    }

    #[test]
    fn json_key_order_is_preserved() {
        let r = CheckpointResult {
            success: true,
            checkpoint_name: "baseline".into(),
            captured_files: vec!["a.txt".into()],
            included_untracked: true,
        };
        let json = render_json(&r).unwrap();
        let p_success = json.find("\"success\"").unwrap();
        let p_name = json.find("\"checkpointName\"").unwrap();
        let p_files = json.find("\"capturedFiles\"").unwrap();
        let p_untracked = json.find("\"includedUntracked\"").unwrap();
        assert!(p_success < p_name && p_name < p_files && p_files < p_untracked);
    }
}
