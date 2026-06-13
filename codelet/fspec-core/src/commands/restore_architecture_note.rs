//! `restore-architecture-note` — Rust port of
//! `src/commands/restore-architecture-note.ts` (RPC-287).
//!
//! Restores a soft-deleted architecture note on a work unit by its STABLE
//! ID (the `index` arg in TS is treated as a note ID — `n.id === index`).
//!
//! Unlike `restore-question`, this command:
//!   * Does NOT enforce a status gate (TS source has no `status` check —
//!     restoration is allowed regardless of work unit status).
//!   * Updates BOTH `workUnit.updatedAt` AND `data.meta.lastUpdated`
//!     (TS L69-74).
//!
//! Idempotent: if the target note is already active (`deleted=false`),
//! returns a success payload with `message: "Item ID <id> already active"`
//! and performs NO disk write (TS L52-60).
//!
//! Reuses existing shared infrastructure:
//! * [`crate::io::ensure::ensure_work_units_file`] — auto-create + load.
//! * [`crate::io::locked_file::write_json_atomic`] — single atomic write.
//! * [`crate::io::time::iso8601_now`] — millisecond ISO-8601 timestamps.
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge
//! at `codelet/fspec/src/restore_architecture_note.rs` is JSON marshalling
//! only.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

/// CLI arguments accepted by `restore-architecture-note`. Mirrors the TS
/// `RestoreArchitectureNoteOptions` interface at
/// `src/commands/restore-architecture-note.ts:9-13`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestoreArchitectureNoteArgs {
    work_unit_id: String,
    index: u64,
}

#[derive(Debug, Serialize)]
struct RestoreArchitectureNoteResult {
    success: bool,
    #[serde(rename = "restoredNote")]
    restored_note: String,
    #[serde(rename = "activeCount")]
    active_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RestoreArchitectureNoteArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "restore-architecture-note",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Load (auto-create on first run).
    let mut data = ensure_work_units_file(project_root)?;

    // Validate work unit exists (TS L31-33). NOTE: no status gate.
    if !data.work_units.contains_key(&args.work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "restore-architecture-note",
            reason: format!("Work unit '{}' does not exist", args.work_unit_id),
        });
    }

    // Locate architectureNotes (immutable scan) (TS L38-42).
    let notes = data
        .work_units
        .get(&args.work_unit_id)
        .and_then(|wu| wu.extra.get("architectureNotes"))
        .and_then(Value::as_array);
    let notes = match notes {
        Some(arr) if !arr.is_empty() => arr.clone(),
        _ => {
            return Err(FspecCoreError::InvalidArgs {
                command: "restore-architecture-note",
                reason: format!(
                    "Work unit '{}' has no architecture notes",
                    args.work_unit_id
                ),
            });
        }
    };

    // Find by stable id (TS L46-50).
    let pos = notes
        .iter()
        .position(|n| n.get("id").and_then(Value::as_u64) == Some(args.index));
    let pos = match pos {
        Some(p) => p,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "restore-architecture-note",
                reason: format!("Architecture note with ID {} not found", args.index),
            });
        }
    };

    let n = &notes[pos];
    let restored_text = n
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let already_active = !n.get("deleted").and_then(Value::as_bool).unwrap_or(false);

    let count_active = |arr: &[Value]| -> usize {
        arr.iter()
            .filter(|n| !n.get("deleted").and_then(Value::as_bool).unwrap_or(false))
            .count()
    };

    // Idempotent path (TS L52-60). NO disk write.
    if already_active {
        let result = RestoreArchitectureNoteResult {
            success: true,
            restored_note: restored_text,
            active_count: count_active(&notes),
            message: Some(format!("Item ID {} already active", args.index)),
        };
        return serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
            command: "restore-architecture-note",
            reason: format!("failed to serialize result: {e}"),
        });
    }

    // Mutate: restore in place (TS L62-64).
    let now = iso8601_now();
    let wu =
        data.work_units
            .get_mut(&args.work_unit_id)
            .ok_or_else(|| FspecCoreError::InvalidArgs {
                command: "restore-architecture-note",
                reason: format!("Work unit '{}' does not exist", args.work_unit_id),
            })?;

    let entry = wu
        .extra
        .get_mut("architectureNotes")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "restore-architecture-note",
            reason: format!(
                "Work unit '{}' has no architecture notes",
                args.work_unit_id
            ),
        })?;
    if let Some(n_mut) = entry.get_mut(pos).and_then(Value::as_object_mut) {
        n_mut.insert("deleted".to_string(), Value::Bool(false));
        n_mut.remove("deletedAt");
    }
    let active = count_active(entry);

    // Bump BOTH workUnit.updatedAt AND meta.lastUpdated (TS L69-74).
    wu.updated_at = now.clone();
    if let Some(meta) = data.meta.as_mut() {
        meta.last_updated = now;
    }

    // Single atomic write (parity with TS fileManager.transaction).
    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data).map_err(|e| match e {
        FspecCoreError::Io { source, .. } => FspecCoreError::Io {
            command: "restore-architecture-note",
            source,
        },
        other => other,
    })?;

    let result = RestoreArchitectureNoteResult {
        success: true,
        restored_note: restored_text,
        active_count: active,
        message: None,
    };
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "restore-architecture-note",
        reason: format!("failed to serialize result: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::useless_vec
    )]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: RestoreArchitectureNoteArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","index":2}"#).unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert_eq!(a.index, 2);
    }

    #[test]
    fn args_parse_fails_without_work_unit_id() {
        let err =
            serde_json::from_str::<RestoreArchitectureNoteArgs>(r#"{"index":0}"#).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("workunitid"),
            "missing-field error must mention workUnitId; got: {msg}"
        );
    }

    #[test]
    fn result_omits_message_when_none() {
        let r = RestoreArchitectureNoteResult {
            success: true,
            restored_note: "N".to_string(),
            active_count: 1,
            message: None,
        };
        let s = serde_json::to_string(&r).unwrap();
        assert!(!s.contains("message"), "got: {s}");
        assert!(s.contains("\"restoredNote\":\"N\""), "got: {s}");
        assert!(s.contains("\"activeCount\":1"), "got: {s}");
    }

    #[test]
    fn result_includes_message_when_set() {
        let r = RestoreArchitectureNoteResult {
            success: true,
            restored_note: "N".to_string(),
            active_count: 1,
            message: Some("Item ID 0 already active".to_string()),
        };
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains("\"message\""), "got: {s}");
        assert!(s.contains("Item ID 0 already active"), "got: {s}");
    }
}
