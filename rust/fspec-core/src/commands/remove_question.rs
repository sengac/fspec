//! `remove-question` — Rust port of `src/commands/remove-question.ts` (RPC-278).
//!
//! Soft-deletes a question on a work unit by its STABLE ID (the `index`
//! arg in TS is actually treated as a question ID — `q.id === index`).
//! The work unit must exist and be in `specifying` status; the
//! `questions` array must exist and be non-empty.
//!
//! Idempotent: if the target question is already `deleted: true`, returns
//! a success payload with `message: "Item ID <id> already deleted"` and
//! performs NO disk write.
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
//! at `rust/fspec/src/remove_question.rs` is JSON marshalling only.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

/// CLI arguments accepted by `remove-question`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoveQuestionArgs {
    work_unit_id: String,
    /// Question index. The TS CLI passes `parseInt(input, 10)` which yields
    /// `NaN` for non-numeric input. The dispatcher accepts either a JSON
    /// integer (canonical case) OR the literal string `"NaN"` (forwarded
    /// by the Rust CLI bridge to preserve TS parity — see
    /// `rust/fspec/src/remove_question.rs::parse_ts_int_radix10`). A
    /// `"NaN"` value never matches an integer question id and surfaces the
    /// canonical `Question with ID NaN not found` error.
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

#[derive(Debug, Serialize)]
struct RemoveQuestionResult {
    success: bool,
    #[serde(rename = "removedQuestion")]
    removed_question: String,
    #[serde(rename = "remainingCount")]
    remaining_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RemoveQuestionArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "remove-question",
            reason: format!("failed to parse args: {e}"),
        })?;

    let mut data = ensure_work_units_file(project_root)?;

    // Validate work unit exists.
    if !data.work_units.contains_key(&args.work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "remove-question",
            reason: format!("Work unit '{}' does not exist", args.work_unit_id),
        });
    }

    // Validate status (immutable borrow).
    let status_str = data
        .work_units
        .get(&args.work_unit_id)
        .map(|w| w.status.as_str())
        .unwrap_or("");
    if status_str != "specifying" {
        return Err(FspecCoreError::InvalidArgs {
            command: "remove-question",
            reason: format!(
                "Can only remove questions during discovery/specification phase. {} is in '{}' state.",
                args.work_unit_id, status_str
            ),
        });
    }

    // Locate the questions array (immutable scan).
    let questions = data
        .work_units
        .get(&args.work_unit_id)
        .and_then(|wu| wu.extra.get("questions"))
        .and_then(Value::as_array);
    let questions = match questions {
        Some(arr) if !arr.is_empty() => arr.clone(),
        _ => {
            return Err(FspecCoreError::InvalidArgs {
                command: "remove-question",
                reason: format!("Work unit {} has no questions", args.work_unit_id),
            });
        }
    };

    // Find by stable id. TS: `q => q.id === options.index`. When index is
    // NaN (non-numeric CLI input), `as_u64()` returns None and no question
    // matches → canonical not-found error.
    let target_id: Option<u64> = args.index.as_u64();
    let pos = match target_id {
        Some(id) => questions
            .iter()
            .position(|q| q.get("id").and_then(Value::as_u64) == Some(id)),
        None => None,
    };
    let pos = match pos {
        Some(p) => p,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "remove-question",
                reason: format!("Question with ID {} not found", args.index.display()),
            });
        }
    };

    let q = &questions[pos];
    let removed_text = q
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let already_deleted = q.get("deleted").and_then(Value::as_bool).unwrap_or(false);

    // Compute remainingCount in either path.
    let count_non_deleted = |arr: &[Value]| -> usize {
        arr.iter()
            .filter(|q| !q.get("deleted").and_then(Value::as_bool).unwrap_or(false))
            .count()
    };

    if already_deleted {
        // Idempotent path — no disk write.
        let result = RemoveQuestionResult {
            success: true,
            removed_question: removed_text,
            remaining_count: count_non_deleted(&questions),
            message: Some(format!("Item ID {} already deleted", args.index.display())),
        };
        return serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
            command: "remove-question",
            reason: format!("failed to serialize result: {e}"),
        });
    }

    // Mutate: soft-delete in place.
    let now = iso8601_now();
    let mut remaining = 0_usize;
    if let Some(wu) = data.work_units.get_mut(&args.work_unit_id) {
        if let Some(entry) = wu.extra.get_mut("questions").and_then(Value::as_array_mut) {
            if let Some(q_mut) = entry.get_mut(pos).and_then(Value::as_object_mut) {
                q_mut.insert("deleted".to_string(), Value::Bool(true));
                q_mut.insert("deletedAt".to_string(), Value::String(now.clone()));
            }
            // Snapshot for the post-write remainingCount.
            remaining = count_non_deleted(entry);
        }
        wu.updated_at = now;
    }

    // Single atomic write.
    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    let result = RemoveQuestionResult {
        success: true,
        removed_question: removed_text,
        remaining_count: remaining,
        message: None,
    };
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "remove-question",
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
        let a: RemoveQuestionArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","index":3}"#).unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert!(matches!(a.index, TsIndex::Int(3)));
    }

    #[test]
    fn args_parse_nan_string() {
        let a: RemoveQuestionArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","index":"NaN"}"#).unwrap();
        assert!(matches!(a.index, TsIndex::Nan));
        assert_eq!(a.index.display(), "NaN");
    }

    #[test]
    fn args_parse_fails_without_work_unit_id() {
        let err = serde_json::from_str::<RemoveQuestionArgs>(r#"{"index":0}"#).unwrap_err();
        let msg = err.to_string();
        assert!(msg.to_lowercase().contains("workunitid"), "got: {msg}");
    }

    #[test]
    fn args_parse_fails_without_index() {
        let err =
            serde_json::from_str::<RemoveQuestionArgs>(r#"{"workUnitId":"AUTH-001"}"#).unwrap_err();
        let msg = err.to_string();
        assert!(msg.to_lowercase().contains("index"), "got: {msg}");
    }

    #[test]
    fn result_omits_message_when_none() {
        let r = RemoveQuestionResult {
            success: true,
            removed_question: "Q?".to_string(),
            remaining_count: 0,
            message: None,
        };
        let s = serde_json::to_string(&r).unwrap();
        assert!(!s.contains("message"), "got: {s}");
    }

    #[test]
    fn result_includes_message_when_set() {
        let r = RemoveQuestionResult {
            success: true,
            removed_question: "Q?".to_string(),
            remaining_count: 1,
            message: Some("Item ID 0 already deleted".to_string()),
        };
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains("\"message\""), "got: {s}");
        assert!(s.contains("Item ID 0 already deleted"), "got: {s}");
    }
}
