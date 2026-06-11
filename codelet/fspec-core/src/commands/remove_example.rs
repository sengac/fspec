//! `remove-example` — Rust port of `src/commands/remove-example.ts` (RPC-273).
//!
//! Soft-deletes an Example Mapping example on a work unit during its
//! specifying phase. Locates the target by **stable id** (NOT by array
//! position), tolerates an already-deleted state as an idempotent no-op
//! (returning success WITHOUT touching disk), and otherwise marks
//! `deleted = true` + `deletedAt = ISO timestamp`, persisting via an
//! atomic write.
//!
//! Both invocation paths (the LLM-facing dispatcher AND the standalone
//! fspec Rust binary's clap subcommand) call this single function —
//! RPC-003 §7/§11 two-front-doors invariant.
//!
//! ## Behavioural parity with `src/commands/remove-example.ts`
//!
//! 1. Resolve work-units file via [`ensure_work_units_file`] — auto-creates
//!    `spec/work-units.json` on first run.
//! 2. Validate work unit exists: `Work unit '<id>' does not exist`.
//! 3. Validate status is `specifying`:
//!    `Can only remove examples during discovery/specification phase. <id> is in '<state>' state.`
//! 4. Validate `examples` array exists AND is non-empty:
//!    `Work unit <id> has no examples`.
//! 5. Locate the target by `examples[i].id === index`:
//!    `Example with ID <index> not found`.
//! 6. **Idempotent re-delete**: if already `deleted == true`, return
//!    success rendering the same `✓ Removed example: "<text>"` line the
//!    TS CLI emits (TS sets a `message` field on the result object but
//!    its `registerRemoveExampleCommand` action handler ignores it and
//!    always prints `removedExample`, so we mirror that surface) and DO
//!    NOT mutate disk.
//! 7. Soft-delete: set `deleted = true`, `deletedAt = iso8601_now()`.
//! 8. Bump `workUnit.updatedAt = iso8601_now()`.
//! 9. Atomic write via [`write_json_atomic`].
//! 10. Render `✓ Removed example: "<text>"` (text captured BEFORE marking
//!     deleted, so the user sees the example body even though it's now
//!     soft-deleted).

use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoveExampleArgs {
    work_unit_id: String,
    /// Example index. The TS CLI passes `parseInt(input, 10)` which yields
    /// `NaN` for non-numeric input. The dispatcher accepts either a JSON
    /// integer (the canonical case) OR the literal string `"NaN"` (forwarded
    /// by the Rust CLI bridge to preserve TS-runtime parity — see
    /// `codelet/fspec/src/remove_example.rs::parse_ts_int_radix10`). A
    /// `"NaN"` value never matches an integer example id and surfaces the
    /// canonical `Example with ID NaN not found` error.
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

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RemoveExampleArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "remove-example",
            reason: format!("failed to parse args: {e}"),
        })?;

    let mut data = ensure_work_units_file(project_root)?;

    // Work-unit-exists gate.
    if !data.work_units.contains_key(&args.work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "remove-example",
            reason: format!("Work unit '{}' does not exist", args.work_unit_id),
        });
    }

    // Status gate.
    let status_str = data
        .work_units
        .get(&args.work_unit_id)
        .expect("present")
        .status
        .as_str();
    if status_str != "specifying" {
        return Err(FspecCoreError::InvalidArgs {
            command: "remove-example",
            reason: format!(
                "Can only remove examples during discovery/specification phase. {} is in '{}' state.",
                args.work_unit_id, status_str
            ),
        });
    }

    // Locate-by-id, examples array check.
    let (located_index, was_deleted, removed_text) = {
        let wu = data
            .work_units
            .get(&args.work_unit_id)
            .expect("present");

        let examples_val = wu.extra.get("examples");
        let arr = match examples_val.and_then(Value::as_array) {
            Some(a) if !a.is_empty() => a,
            _ => {
                return Err(FspecCoreError::InvalidArgs {
                    command: "remove-example",
                    reason: format!("Work unit {} has no examples", args.work_unit_id),
                });
            }
        };

        // Find by stable id. When `index` is NaN (TS `parseInt` of non-numeric
        // input), no integer example id ever satisfies strict equality and the
        // canonical `Example with ID NaN not found` error is surfaced.
        let target_id: Option<i64> = match args.index {
            TsIndex::Int(n) => Some(n),
            TsIndex::Nan => None,
        };

        let mut found: Option<(usize, bool, String)> = None;
        if let Some(target) = target_id {
            for (i, item) in arr.iter().enumerate() {
                let id = item.get("id").and_then(Value::as_i64);
                if id == Some(target) {
                    let deleted = item
                        .get("deleted")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let text = item
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    found = Some((i, deleted, text));
                    break;
                }
            }
        }
        match found {
            Some(v) => v,
            None => {
                return Err(FspecCoreError::InvalidArgs {
                    command: "remove-example",
                    reason: format!("Example with ID {} not found", args.index.display()),
                });
            }
        }
    };

    // Idempotent re-delete path — return success without writing.
    // TS sets a `message` field on the result ('Item ID <n> already deleted')
    // BUT the Commander.js action handler at
    // src/commands/remove-example.ts:100-102 ignores it and always prints
    // `✓ Removed example: "<text>"`. We mirror the surface, not the internal
    // result shape, because the CLI byte parity is the contract.
    if was_deleted {
        return Ok(format!("✓ Removed example: \"{}\"\n", removed_text));
    }

    // Mutate: set deleted + deletedAt, bump updatedAt, write atomically.
    let now_ts = iso8601_now();
    {
        let wu = data
            .work_units
            .get_mut(&args.work_unit_id)
            .expect("present");
        if let Some(arr) = wu
            .extra
            .get_mut("examples")
            .and_then(Value::as_array_mut)
        {
            if let Some(item) = arr.get_mut(located_index).and_then(Value::as_object_mut) {
                item.insert("deleted".to_string(), Value::Bool(true));
                item.insert("deletedAt".to_string(), Value::String(now_ts.clone()));
            }
        }
        wu.updated_at = now_ts.clone();
    }

    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    Ok(format!("✓ Removed example: \"{}\"\n", removed_text))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;

    #[test]
    fn args_parse_with_camel_case() {
        let a: RemoveExampleArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","index":3}"#).unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert!(matches!(a.index, TsIndex::Int(3)));
    }

    #[test]
    fn args_parse_accepts_nan_string_for_ts_parity() {
        let a: RemoveExampleArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","index":"NaN"}"#).unwrap();
        assert!(matches!(a.index, TsIndex::Nan));
        assert_eq!(a.index.display(), "NaN");
    }

    #[test]
    fn args_parse_fails_without_index() {
        let err = serde_json::from_str::<RemoveExampleArgs>(r#"{"workUnitId":"AUTH-001"}"#).unwrap_err();
        let msg = err.to_string();
        assert!(msg.to_lowercase().contains("index"), "got: {msg}");
    }
}
