//! `remove-architecture-note` — Rust port of `src/commands/remove-architecture-note.ts` (RPC-267).
//!
//! Soft-deletes a stable-ID architecture note on a work unit. The TS source
//! treats the positional `index` argument as the note's stable numeric ID
//! (see TS L45-50 — "index is now treated as ID for stable indices"), not
//! an array position. We mirror that semantic exactly.
//!
//! Idempotent already-deleted path: when the matched note already has
//! `deleted=true`, the dispatcher returns success WITHOUT mutating disk and
//! surfaces the canonical `"Item ID <id> already deleted"` message. Mirrors
//! TS L52-60.
//!
//! Reuses existing shared infrastructure:
//! * [`crate::io::ensure::ensure_work_units_file`] — auto-create + load
//!   `spec/work-units.json`.
//! * [`crate::io::locked_file::write_json_atomic`] — single atomic write.
//! * [`crate::io::time::iso8601_now`] — millisecond-precision ISO-8601
//!   timestamps (used for the soft-delete `deletedAt` field).
//!
//! Two-front-doors: both the LLM dispatcher and the standalone Rust CLI
//! invoke this single function (RPC-003 §7/§11). The CLI bridge at
//! `codelet/fspec/src/remove_architecture_note.rs` is JSON marshalling only.

use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

/// CLI arguments accepted by `remove-architecture-note`. Mirrors the TS
/// `RemoveArchitectureNoteOptions` interface at
/// `src/commands/remove-architecture-note.ts:9-13`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoveArchitectureNoteArgs {
    work_unit_id: String,
    /// The note's stable numeric ID (NOT an array position). TS uses
    /// `parseInt` on the Commander positional and then performs an
    /// `id === options.index` lookup; we mirror that. We accept either
    /// a JSON integer (canonical case) OR the literal string `"NaN"`
    /// (forwarded by the Rust CLI bridge to preserve TS `parseInt(_, 10)`
    /// semantics for non-numeric input). A `"NaN"` value never matches an
    /// integer id and surfaces the canonical
    /// `Architecture note with ID NaN not found` error.
    #[serde(deserialize_with = "deserialize_ts_index")]
    index: TsIndex,
}

/// TS-`parseInt(_, 10)` result: either a finite integer or NaN.
#[derive(Debug, Clone, Copy)]
enum TsIndex {
    Int(i64),
    Nan,
}

impl TsIndex {
    fn display(&self) -> String {
        match self {
            TsIndex::Int(n) => n.to_string(),
            TsIndex::Nan => "NaN".to_string(),
        }
    }

    fn as_u64(&self) -> Option<u64> {
        match self {
            TsIndex::Int(n) if *n >= 0 => Some(*n as u64),
            _ => None,
        }
    }
}

fn deserialize_ts_index<'de, D>(de: D) -> Result<TsIndex, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{Error, Unexpected};
    let v = Value::deserialize(de)?;
    match &v {
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(TsIndex::Int(i))
            } else {
                Ok(TsIndex::Nan)
            }
        }
        Value::String(s) if s == "NaN" => Ok(TsIndex::Nan),
        Value::Null => Ok(TsIndex::Nan),
        other => Err(D::Error::invalid_type(
            Unexpected::Other(&format!("{other:?}")),
            &"integer or \"NaN\"",
        )),
    }
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RemoveArchitectureNoteArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "remove-architecture-note",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Load (auto-create on first run).
    let mut data = ensure_work_units_file(project_root)?;

    // Validate work unit exists (TS L31-33).
    let wu = match data.work_units.get_mut(&args.work_unit_id) {
        Some(w) => w,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "remove-architecture-note",
                reason: format!("Work unit '{}' does not exist", args.work_unit_id),
            });
        }
    };

    // Validate architectureNotes exists AND is non-empty (TS L38-42).
    let notes_value = wu.extra.get_mut("architectureNotes");
    let notes_array = match notes_value {
        Some(Value::Array(arr)) if !arr.is_empty() => arr,
        _ => {
            return Err(FspecCoreError::InvalidArgs {
                command: "remove-architecture-note",
                reason: format!(
                    "Work unit '{}' has no architecture notes",
                    args.work_unit_id
                ),
            });
        }
    };

    // Locate the note by stable ID (TS L45-50). When index is NaN
    // (non-numeric CLI input), `as_u64()` returns None and no note matches.
    let target_id: Option<u64> = args.index.as_u64();
    let position = match target_id {
        Some(id) => notes_array.iter().position(|n| {
            n.get("id")
                .and_then(Value::as_u64)
                .map(|nid| nid == id)
                .unwrap_or(false)
        }),
        None => None,
    };
    let position = match position {
        Some(p) => p,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "remove-architecture-note",
                reason: format!(
                    "Architecture note with ID {} not found",
                    args.index.display()
                ),
            });
        }
    };

    // Idempotent already-deleted path (TS L52-60).
    let note = &notes_array[position];
    let already_deleted = note
        .get("deleted")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if already_deleted {
        return Ok(render_success(target_id));
    }

    // Soft-delete: set deleted=true and deletedAt=iso8601_now() (TS L62-64).
    let now = iso8601_now();
    let note_obj =
        notes_array[position]
            .as_object_mut()
            .ok_or_else(|| FspecCoreError::ParseJson {
                file: "work-units.json".to_string(),
                reason: "architecture note must be an object".to_string(),
            })?;
    note_obj.insert("deleted".to_string(), Value::Bool(true));
    note_obj.insert("deletedAt".to_string(), Value::String(now.clone()));

    // Bump work-unit updatedAt and top-level meta.lastUpdated (TS L69-74).
    wu.updated_at = now.clone();
    if let Some(meta) = data.meta.as_mut() {
        meta.last_updated = now;
    }

    // Single atomic write (parity with TS fileManager.transaction).
    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data).map_err(|e| match e {
        FspecCoreError::Io { source, .. } => FspecCoreError::Io {
            command: "remove-architecture-note",
            source,
        },
        other => other,
    })?;

    Ok(render_success(None))
}

/// Render the success block. When `idempotent_id` is `Some(id)`, append the
/// secondary indented `Item ID <id> already deleted` line — mirroring the
/// TS CLI wrapper at L97-99.
fn render_success(idempotent_id: Option<u64>) -> String {
    let mut out = String::new();
    out.push_str("✓ Architecture note removed successfully\n");
    if let Some(id) = idempotent_id {
        out.push_str(&format!("  Item ID {id} already deleted\n"));
    }
    out
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
        let a: RemoveArchitectureNoteArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","index":2}"#).unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert!(matches!(a.index, TsIndex::Int(2)));
    }

    #[test]
    fn args_parse_nan_string() {
        let a: RemoveArchitectureNoteArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","index":"NaN"}"#).unwrap();
        assert!(matches!(a.index, TsIndex::Nan));
        assert_eq!(a.index.display(), "NaN");
    }

    #[test]
    fn args_parse_fails_without_work_unit_id() {
        let err = serde_json::from_str::<RemoveArchitectureNoteArgs>(r#"{"index":0}"#).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("workunitid"),
            "missing-field error must mention workUnitId; got: {msg}"
        );
    }

    #[test]
    fn render_success_basic() {
        let out = render_success(None);
        assert!(out.contains("✓ Architecture note removed successfully"));
        assert!(!out.contains("already deleted"));
    }

    #[test]
    fn render_success_idempotent_appends_indented_message() {
        let out = render_success(Some(0));
        assert!(out.contains("✓ Architecture note removed successfully"));
        assert!(out.contains("  Item ID 0 already deleted"));
    }
}
