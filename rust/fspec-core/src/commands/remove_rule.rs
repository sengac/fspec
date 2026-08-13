//! `remove-rule` — Rust port of `src/commands/remove-rule.ts` (RPC-279).
//!
//! Soft-deletes a rule (identified by its stable `id` field) on a work unit
//! in the specifying phase. The rule is NOT physically removed — instead its
//! `deleted` flag is set to `true` and a `deletedAt` timestamp is added.
//! Already-deleted rules return an idempotent success WITHOUT writing to disk.
//!
//! Reuses existing shared infrastructure:
//! * [`crate::io::ensure::ensure_work_units_file`]
//! * [`crate::io::locked_file::write_json_atomic`]
//! * [`crate::io::time::iso8601_now`]
//!
//! Two-front-doors: bridge marshals JSON `{workUnitId, index}` and forwards
//! to this single source-of-truth function.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoveRuleArgs {
    work_unit_id: String,
    /// Rule index. The TS CLI passes `parseInt(input, 10)` which yields
    /// `NaN` (a JSON number with non-finite value, but JSON.stringify
    /// converts NaN → null) for non-numeric input. The dispatcher accepts
    /// either a JSON integer (the canonical case) OR the literal string
    /// `"NaN"` (forwarded by the Rust CLI bridge to preserve TS-runtime
    /// parity — see `rust/fspec/src/remove_rule.rs::parse_ts_int_radix10`).
    /// A `"NaN"` value never matches an integer rule id and surfaces the
    /// canonical `Rule with ID NaN not found` error.
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
                // Non-integer number (float or out-of-range) → NaN for parity.
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
struct RemoveRuleResult {
    success: bool,
    #[serde(rename = "removedRule")]
    removed_rule: String,
    #[serde(rename = "remainingCount")]
    remaining_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RemoveRuleArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "remove-rule",
            reason: format!("failed to parse args: {e}"),
        })?;

    let mut data = ensure_work_units_file(project_root)?;

    // Validate work unit exists.
    if !data.work_units.contains_key(&args.work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "remove-rule",
            reason: format!("Work unit '{}' does not exist", args.work_unit_id),
        });
    }

    // Validate specifying status.
    let status_str = data
        .work_units
        .get(&args.work_unit_id)
        .map(|w| w.status.as_str())
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "remove-rule",
            reason: format!("Work unit '{}' does not exist", args.work_unit_id),
        })?;
    if status_str != "specifying" {
        return Err(FspecCoreError::InvalidArgs {
            command: "remove-rule",
            reason: format!(
                "Can only remove rules during discovery/specification phase. {} is in '{}' state.",
                args.work_unit_id, status_str
            ),
        });
    }

    // Validate rules array exists and non-empty.
    let wu =
        data.work_units
            .get_mut(&args.work_unit_id)
            .ok_or_else(|| FspecCoreError::InvalidArgs {
                command: "remove-rule",
                reason: format!("Work unit '{}' does not exist", args.work_unit_id),
            })?;
    let rules_present = matches!(wu.extra.get("rules"), Some(Value::Array(a)) if !a.is_empty());
    if !rules_present {
        return Err(FspecCoreError::InvalidArgs {
            command: "remove-rule",
            reason: format!("Work unit {} has no rules", args.work_unit_id),
        });
    }

    let rules = match wu.extra.get_mut("rules") {
        Some(Value::Array(a)) => a,
        _ => unreachable!("checked above"),
    };

    // Find rule by stable id (linear scan). TS uses `find(r => r.id === options.index)`.
    // When `index` is NaN (TS `parseInt` of non-numeric input), no integer
    // rule id ever satisfies strict equality and the canonical
    // `Rule with ID NaN not found` error is surfaced — byte parity with TS.
    let target_id: Option<i64> = match args.index {
        TsIndex::Int(n) => Some(n),
        TsIndex::Nan => None,
    };
    let pos = target_id.and_then(|id| {
        rules
            .iter()
            .position(|r| r.get("id").and_then(Value::as_i64) == Some(id))
    });
    let pos = match pos {
        Some(p) => p,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "remove-rule",
                reason: format!("Rule with ID {} not found", args.index.display()),
            });
        }
    };

    let rule = &mut rules[pos];

    let text = rule
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let already_deleted = rule
        .get("deleted")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    // Idempotent path: already-deleted rule returns success WITHOUT disk write.
    if already_deleted {
        let remaining = non_deleted_count(rules);
        let result = RemoveRuleResult {
            success: true,
            removed_rule: text,
            remaining_count: remaining,
            message: Some(format!("Item ID {} already deleted", args.index.display())),
        };
        return serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
            command: "remove-rule",
            reason: format!("failed to serialize result: {e}"),
        });
    }

    // Soft-delete: set deleted=true and deletedAt=now.
    let now = iso8601_now();
    if let Value::Object(obj) = rule {
        obj.insert("deleted".to_string(), Value::Bool(true));
        obj.insert("deletedAt".to_string(), Value::String(now.clone()));
    }

    // Compute remaining count AFTER mutation.
    let remaining = non_deleted_count(rules);

    // Bump updatedAt.
    wu.updated_at = now;

    // Atomic write.
    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    let result = RemoveRuleResult {
        success: true,
        removed_rule: text,
        remaining_count: remaining,
        message: None,
    };
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "remove-rule",
        reason: format!("failed to serialize result: {e}"),
    })
}

fn non_deleted_count(rules: &[Value]) -> usize {
    rules
        .iter()
        .filter(|r| !r.get("deleted").and_then(Value::as_bool).unwrap_or(false))
        .count()
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
    use serde_json::json;

    #[test]
    fn args_parse_camel_case() {
        let a: RemoveRuleArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","index":3}"#).unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert!(matches!(a.index, TsIndex::Int(3)));
    }

    #[test]
    fn args_parse_accepts_nan_string_for_ts_parity() {
        let a: RemoveRuleArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","index":"NaN"}"#).unwrap();
        assert!(matches!(a.index, TsIndex::Nan));
        assert_eq!(a.index.display(), "NaN");
    }

    #[test]
    fn non_deleted_count_filters_out_deleted_true() {
        let rules = vec![
            json!({"id":0,"deleted":false}),
            json!({"id":1,"deleted":true}),
            json!({"id":2,"deleted":false}),
        ];
        assert_eq!(non_deleted_count(&rules), 2);
    }
}
