//! `restore-rule` — Rust port of `src/commands/restore-rule.ts` (RPC-291).
//!
//! Un-deletes a soft-deleted business rule on a work unit in the
//! specifying phase. Two dispatcher branches:
//!
//! * **Single restore** (the CLI-exposed path): `args = {workUnitId, index}`.
//!   Mirrors `restore-example`: validate, locate-by-stable-id, idempotent
//!   already-active, otherwise clear `deleted` and REMOVE `deletedAt`.
//! * **Bulk restore** (dispatcher-only — the CLI `--ids` flag is advertised
//!   in the help fixture but the TS `registerRestoreRuleCommand` does NOT
//!   wire it as a clap option, and we preserve that asymmetry verbatim in
//!   `rust/fspec/src/restore_rule.rs`): `args = {workUnitId, ids}` where
//!   `ids` is a comma-separated string. Atomically validates every id
//!   exists; if any is unknown the entire call errors WITHOUT writing
//!   (TS-parity by virtue of throwing before `fileManager.transaction`).
//!   Already-active items in the list are silently skipped and excluded
//!   from the joined output text. The `updatedAt` bump and atomic write
//!   ALWAYS run on the success path even when zero rules were actually
//!   restored (TS at `src/commands/restore-rule.ts:70-74` does the same).
//!
//! When BOTH `ids` and `index` are present the bulk branch wins — the TS
//! `if (options.ids)` check runs first.
//!
//! Output rendering (returned as the dispatcher's `data` string; the CLI
//! bridge prints it verbatim):
//!   - Single happy:        `✓ Restored rule: "<text>"\n`
//!   - Single idempotent:   `✓ Restored rule: "<text>"\n  Item ID <n> already active\n`
//!   - Bulk:                `✓ Restored rule: "<text0>, <text1>, ..."\n`

use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;
use crate::types::work_unit::WorkUnitsData;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RestoreRuleArgs {
    work_unit_id: String,
    /// Comma-separated list of integer ids for bulk restore. When present
    /// the bulk branch wins over `index` (TS `if (options.ids)` check).
    #[serde(default)]
    ids: Option<String>,
    /// Single-restore rule index. The TS CLI passes `parseInt(input, 10)`
    /// which yields `NaN` for non-numeric input. The dispatcher accepts
    /// either a JSON integer (the canonical case) OR the literal string
    /// `"NaN"` (forwarded by the Rust CLI bridge to preserve TS-runtime
    /// parity — see `rust/fspec/src/restore_rule.rs::parse_ts_int_radix10`).
    /// Optional because the bulk branch ignores it.
    #[serde(default, deserialize_with = "deserialize_ts_index_opt")]
    index: Option<TsIndex>,
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

fn deserialize_ts_index_opt<'de, D>(de: D) -> Result<Option<TsIndex>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{Error, Unexpected};
    let v = Option::<Value>::deserialize(de)?;
    let Some(v) = v else { return Ok(None) };
    match &v {
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Some(TsIndex::Int(i)))
            } else {
                Ok(Some(TsIndex::Nan))
            }
        }
        Value::String(s) if s == "NaN" => Ok(Some(TsIndex::Nan)),
        Value::Null => Ok(None),
        other => Err(D::Error::invalid_type(
            Unexpected::Other(&format!("{other:?}")),
            &"integer or \"NaN\"",
        )),
    }
}

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RestoreRuleArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "restore-rule",
            reason: format!("failed to parse args: {e}"),
        })?;

    let mut data = ensure_work_units_file(project_root)?;

    // Work-unit-exists gate.
    if !data.work_units.contains_key(&args.work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "restore-rule",
            reason: format!("Work unit '{}' does not exist", args.work_unit_id),
        });
    }

    // Status gate.
    let status_str = data
        .work_units
        .get(&args.work_unit_id)
        .map(|w| w.status.as_str())
        .unwrap_or("");
    if status_str != "specifying" {
        return Err(FspecCoreError::InvalidArgs {
            command: "restore-rule",
            reason: format!(
                "Can only restore rules during discovery/specification phase. {} is in '{}' state.",
                args.work_unit_id, status_str
            ),
        });
    }

    // Rules-array-present gate.
    {
        let rules_present = data
            .work_units
            .get(&args.work_unit_id)
            .map(|wu| {
                matches!(
                    wu.extra.get("rules"),
                    Some(Value::Array(a)) if !a.is_empty()
                )
            })
            .unwrap_or(false);
        if !rules_present {
            return Err(FspecCoreError::InvalidArgs {
                command: "restore-rule",
                reason: format!("Work unit {} has no rules", args.work_unit_id),
            });
        }
    }

    // BULK BRANCH — `ids` wins over `index` when both are present.
    if let Some(ids_str) = args.ids.as_deref() {
        return run_bulk(&mut data, &args.work_unit_id, ids_str, project_root);
    }

    // SINGLE BRANCH.
    let index = args.index.unwrap_or(TsIndex::Nan);
    run_single(&mut data, &args.work_unit_id, index, project_root)
}

fn run_single(
    data: &mut WorkUnitsData,
    work_unit_id: &str,
    index: TsIndex,
    project_root: &Path,
) -> Result<String, FspecCoreError> {
    let located = data
        .work_units
        .get(work_unit_id)
        .and_then(|wu| wu.extra.get("rules"))
        .and_then(Value::as_array)
        .and_then(|arr| {
            let target_id: i64 = match index {
                TsIndex::Int(n) => n,
                TsIndex::Nan => return None,
            };
            arr.iter().enumerate().find_map(|(i, item)| {
                if item.get("id").and_then(Value::as_i64) == Some(target_id) {
                    let deleted = item
                        .get("deleted")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let t = item
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    Some((i, deleted, t))
                } else {
                    None
                }
            })
        });
    let (located_index, was_deleted, text) = match located {
        Some(v) => v,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "restore-rule",
                reason: format!("Rule with ID {} not found", index.display()),
            });
        }
    };

    // Idempotent already-active path — return success WITHOUT writing.
    if !was_deleted {
        return Ok(format!(
            "✓ Restored rule: \"{}\"\n  Item ID {} already active\n",
            text,
            index.display()
        ));
    }

    // Restore: clear deleted, REMOVE deletedAt key, bump updatedAt, write.
    let now_ts = iso8601_now();
    if let Some(wu) = data.work_units.get_mut(work_unit_id) {
        if let Some(arr) = wu.extra.get_mut("rules").and_then(Value::as_array_mut) {
            if let Some(item) = arr.get_mut(located_index).and_then(Value::as_object_mut) {
                item.insert("deleted".to_string(), Value::Bool(false));
                item.remove("deletedAt");
            }
        }
        wu.updated_at = now_ts;
    }

    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, data)?;

    Ok(format!("✓ Restored rule: \"{text}\"\n"))
}

fn run_bulk(
    data: &mut WorkUnitsData,
    work_unit_id: &str,
    ids_str: &str,
    project_root: &Path,
) -> Result<String, FspecCoreError> {
    // TS: `options.ids.split(',').map(id => parseInt(id.trim(), 10))`.
    // We pre-validate every token resolves to a known rule id BEFORE
    // touching the in-memory tree, mirroring the TS atomic-failure
    // promise documented in the help fixture
    // ("Bulk restore validates all IDs before restoring any item").
    let tokens: Vec<&str> = ids_str.split(',').collect();
    let parsed: Vec<TsIndex> = tokens.iter().map(|tok| parse_ts_int(tok.trim())).collect();

    // Collect (position, already_active, text) for each requested id.
    // Errors surface here BEFORE any mutation so the file stays byte-equal.
    let plan: Vec<(usize, bool, String, TsIndex)> = {
        let arr = match data
            .work_units
            .get(work_unit_id)
            .and_then(|wu| wu.extra.get("rules"))
            .and_then(Value::as_array)
        {
            Some(arr) => arr,
            None => {
                return Err(FspecCoreError::InvalidArgs {
                    command: "restore-rule",
                    reason: format!("Work unit {work_unit_id} has no rules"),
                });
            }
        };

        let mut plan = Vec::with_capacity(parsed.len());
        for idx in parsed {
            let target_id: Option<i64> = match idx {
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
                        let t = item
                            .get("text")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        found = Some((i, deleted, t));
                        break;
                    }
                }
            }
            match found {
                Some((i, deleted, t)) => plan.push((i, deleted, t, idx)),
                None => {
                    return Err(FspecCoreError::InvalidArgs {
                        command: "restore-rule",
                        reason: format!("Rule with ID {} not found", idx.display()),
                    });
                }
            }
        }
        plan
    };

    // Apply: mutate each currently-deleted entry, collect restored text.
    // Already-active entries are silently skipped (TS `continue`).
    let now_ts = iso8601_now();
    let mut restored_texts: Vec<String> = Vec::new();
    if let Some(arr) = data
        .work_units
        .get_mut(work_unit_id)
        .and_then(|wu| wu.extra.get_mut("rules"))
        .and_then(Value::as_array_mut)
    {
        for (i, was_deleted, text, _idx) in plan {
            if !was_deleted {
                continue;
            }
            if let Some(item) = arr.get_mut(i).and_then(Value::as_object_mut) {
                item.insert("deleted".to_string(), Value::Bool(false));
                item.remove("deletedAt");
            }
            restored_texts.push(text);
        }
    }
    // TS always bumps updatedAt and writes on the bulk success path,
    // even when zero rules were actually restored.
    if let Some(wu) = data.work_units.get_mut(work_unit_id) {
        wu.updated_at = now_ts;
    }

    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, data)?;

    Ok(format!(
        "✓ Restored rule: \"{}\"\n",
        restored_texts.join(", ")
    ))
}

/// TS `parseInt(token, 10)` for a single bulk-id token. Leading sign
/// allowed; non-numeric input becomes `NaN` which (in the locate-by-id
/// step) surfaces as `Rule with ID NaN not found`.
fn parse_ts_int(raw: &str) -> TsIndex {
    let trimmed = raw.trim_start();
    let (sign, rest) = match trimmed.chars().next() {
        Some('-') => (-1i64, &trimmed[1..]),
        Some('+') => (1i64, &trimmed[1..]),
        _ => (1i64, trimmed),
    };
    if rest.is_empty() || !rest.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return TsIndex::Nan;
    }
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    match digits.parse::<i64>() {
        Ok(n) => TsIndex::Int(sign * n),
        Err(_) => TsIndex::Nan,
    }
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
    fn args_parse_single_with_index() {
        let a: RestoreRuleArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","index":3}"#).unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert!(a.ids.is_none());
        assert!(matches!(a.index, Some(TsIndex::Int(3))));
    }

    #[test]
    fn args_parse_bulk_with_ids() {
        let a: RestoreRuleArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","ids":"0,1,2"}"#).unwrap();
        assert_eq!(a.ids.as_deref(), Some("0,1,2"));
        assert!(a.index.is_none());
    }

    #[test]
    fn args_parse_accepts_nan_string_for_ts_parity() {
        let a: RestoreRuleArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","index":"NaN"}"#).unwrap();
        assert!(matches!(a.index, Some(TsIndex::Nan)));
    }

    #[test]
    fn parse_ts_int_handles_signs_and_non_numeric() {
        assert!(matches!(parse_ts_int("0"), TsIndex::Int(0)));
        assert!(matches!(parse_ts_int("42"), TsIndex::Int(42)));
        assert!(matches!(parse_ts_int("-3"), TsIndex::Int(-3)));
        assert!(matches!(parse_ts_int("abc"), TsIndex::Nan));
        assert!(matches!(parse_ts_int(""), TsIndex::Nan));
    }
}
