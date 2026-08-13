//! `restore-question` — Rust port of `src/commands/restore-question.ts` (RPC-290).
//!
//! Restores a soft-deleted question on a work unit by its STABLE ID (the
//! `index` arg in TS is treated as a question ID — `q.id === index`).
//! The work unit must exist and be in `specifying` status; the `questions`
//! array must exist and be non-empty.
//!
//! Idempotent: if the target question is already active (`deleted=false`),
//! returns a success payload with `message: "Item ID <id> already active"`
//! and performs NO disk write.
//!
//! Unlike `restore-architecture-note`, this command does NOT bump
//! `data.meta.lastUpdated` — only `workUnit.updatedAt`. Mirrors TS L74.
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
//! at `rust/fspec/src/restore_question.rs` is JSON marshalling only.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

/// CLI arguments accepted by `restore-question`. Mirrors the TS
/// `RestoreQuestionOptions` interface at
/// `src/commands/restore-question.ts:9-13`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestoreQuestionArgs {
    work_unit_id: String,
    index: u64,
}

#[derive(Debug, Serialize)]
struct RestoreQuestionResult {
    success: bool,
    #[serde(rename = "restoredQuestion")]
    restored_question: String,
    #[serde(rename = "activeCount")]
    active_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RestoreQuestionArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "restore-question",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Load (auto-create on first run). On a brand-new workspace this
    // writes the canonical empty initial structure to disk before we
    // return the "work unit does not exist" error — parity with TS
    // `ensureWorkUnitsFile(cwd)` semantics.
    let mut data = ensure_work_units_file(project_root)?;

    // Validate work unit exists (TS L31-33).
    if !data.work_units.contains_key(&args.work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "restore-question",
            reason: format!("Work unit '{}' does not exist", args.work_unit_id),
        });
    }

    // Validate status (TS L39-43).
    let status_str = data
        .work_units
        .get(&args.work_unit_id)
        .map(|w| w.status.as_str())
        .unwrap_or("");
    if status_str != "specifying" {
        return Err(FspecCoreError::InvalidArgs {
            command: "restore-question",
            reason: format!(
                "Can only restore questions during discovery/specification phase. {} is in '{}' state.",
                args.work_unit_id, status_str
            ),
        });
    }

    // Locate the questions array (immutable scan) (TS L46-48).
    let questions = data
        .work_units
        .get(&args.work_unit_id)
        .and_then(|wu| wu.extra.get("questions"))
        .and_then(Value::as_array);
    let questions = match questions {
        Some(arr) if !arr.is_empty() => arr.clone(),
        _ => {
            return Err(FspecCoreError::InvalidArgs {
                command: "restore-question",
                reason: format!("Work unit {} has no questions", args.work_unit_id),
            });
        }
    };

    // Find by stable id (TS L50-54).
    let pos = questions
        .iter()
        .position(|q| q.get("id").and_then(Value::as_u64) == Some(args.index));
    let pos = match pos {
        Some(p) => p,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "restore-question",
                reason: format!("Question with ID {} not found", args.index),
            });
        }
    };

    let q = &questions[pos];
    let restored_text = q
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let already_active = !q.get("deleted").and_then(Value::as_bool).unwrap_or(false);

    let count_active = |arr: &[Value]| -> usize {
        arr.iter()
            .filter(|q| !q.get("deleted").and_then(Value::as_bool).unwrap_or(false))
            .count()
    };

    // Idempotent path (TS L57-65). NO disk write.
    if already_active {
        let result = RestoreQuestionResult {
            success: true,
            restored_question: restored_text,
            active_count: count_active(&questions),
            message: Some(format!("Item ID {} already active", args.index)),
        };
        return serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
            command: "restore-question",
            reason: format!("failed to serialize result: {e}"),
        });
    }

    // Mutate: restore in place (TS L68-69, L74).
    let now = iso8601_now();
    let mut active = 0_usize;
    if let Some(wu) = data.work_units.get_mut(&args.work_unit_id) {
        if let Some(entry) = wu.extra.get_mut("questions").and_then(Value::as_array_mut) {
            if let Some(q_mut) = entry.get_mut(pos).and_then(Value::as_object_mut) {
                q_mut.insert("deleted".to_string(), Value::Bool(false));
                q_mut.remove("deletedAt");
            }
            active = count_active(entry);
        }
        wu.updated_at = now;
    }

    // Single atomic write (parity with TS fileManager.transaction).
    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data).map_err(|e| match e {
        FspecCoreError::Io { source, .. } => FspecCoreError::Io {
            command: "restore-question",
            source,
        },
        other => other,
    })?;

    let result = RestoreQuestionResult {
        success: true,
        restored_question: restored_text,
        active_count: active,
        message: None,
    };
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "restore-question",
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
        let a: RestoreQuestionArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","index":3}"#).unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert_eq!(a.index, 3);
    }

    #[test]
    fn args_parse_fails_without_work_unit_id() {
        let err = serde_json::from_str::<RestoreQuestionArgs>(r#"{"index":0}"#).unwrap_err();
        let msg = err.to_string();
        assert!(msg.to_lowercase().contains("workunitid"), "got: {msg}");
    }

    #[test]
    fn result_omits_message_when_none() {
        let r = RestoreQuestionResult {
            success: true,
            restored_question: "Q?".to_string(),
            active_count: 1,
            message: None,
        };
        let s = serde_json::to_string(&r).unwrap();
        assert!(!s.contains("message"), "got: {s}");
        assert!(s.contains("\"restoredQuestion\":\"Q?\""), "got: {s}");
        assert!(s.contains("\"activeCount\":1"), "got: {s}");
    }

    #[test]
    fn result_includes_message_when_set() {
        let r = RestoreQuestionResult {
            success: true,
            restored_question: "Q?".to_string(),
            active_count: 1,
            message: Some("Item ID 0 already active".to_string()),
        };
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains("\"message\""), "got: {s}");
        assert!(s.contains("Item ID 0 already active"), "got: {s}");
    }
}
