//! `add-rule` — Rust port of `src/commands/add-rule.ts` (RPC-189).
//!
//! Appends a [`RuleItem`]-shaped record to a work unit's `rules` array during
//! the specifying phase of Example Mapping. The work unit must exist and be
//! in `specifying` status; otherwise the dispatcher returns a canonical
//! validation error and disk state is left untouched.
//!
//! Reuses existing shared infrastructure:
//! * [`crate::io::ensure::ensure_work_units_file`] — auto-create + load
//!   `spec/work-units.json` (parity with TS `ensureWorkUnitsFile`).
//! * [`crate::io::locked_file::write_json_atomic`] — single atomic write at
//!   the end (the TS implementation uses `fileManager.transaction`).
//! * [`crate::io::time::iso8601_now`] — millisecond-precision ISO-8601
//!   timestamps (parity with TS `new Date().toISOString()`).
//!
//! ## On-disk shape
//!
//! Per the TS `RuleItem` interface (`src/types/index.ts:4-13`), each rule is:
//!
//! ```json
//! {
//!   "id": 0,
//!   "text": "Email must be valid",
//!   "deleted": false,
//!   "createdAt": "2026-06-11T12:00:00.000Z"
//! }
//! ```
//!
//! The `rules` array and the `nextRuleId` counter both live in the work
//! unit's `extra` map (round-tripped via `#[serde(flatten)]` on
//! [`crate::types::work_unit::WorkUnit`]).
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge at
//! `codelet/fspec/src/add_rule.rs` is JSON marshalling only — no domain
//! logic.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

/// CLI arguments accepted by `add-rule`. Mirrors the TS
/// `AddRuleOptions` interface at `src/commands/add-rule.ts:9-13`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddRuleArgs {
    work_unit_id: String,
    rule: String,
}

#[derive(Debug, Serialize)]
struct AddRuleResult {
    success: bool,
    #[serde(rename = "ruleCount")]
    rule_count: usize,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddRuleArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-rule",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Load (auto-create on first run). On a brand-new workspace this writes
    // the canonical empty initial structure to disk before we return the
    // missing-source error below, matching TS `ensureWorkUnitsFile`.
    let mut data = ensure_work_units_file(project_root)?;

    // Validate work unit exists (mirrors src/commands/add-rule.ts:28-30).
    let wu = match data.work_units.get_mut(&args.work_unit_id) {
        Some(wu) => wu,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "add-rule",
                reason: format!("Work unit '{}' does not exist", args.work_unit_id),
            });
        }
    };

    // Validate work unit is in specifying state (mirrors
    // src/commands/add-rule.ts:34-39). We capture the status string BEFORE
    // mutating because the canonical error message embeds the status as
    // its TS-lowercase form.
    let status_str = wu.status.as_str();
    if status_str != "specifying" {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-rule",
            reason: format!(
                "Can only add rules during discovery/specification phase. {} is in '{}' state.",
                args.work_unit_id, status_str
            ),
        });
    }

    let now = iso8601_now();

    // Mutate: ensure `rules` and `nextRuleId` exist on the WorkUnit's
    // extra map, then post-increment the counter and push the new rule.

    let next_id = wu
        .extra
        .get("nextRuleId")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    // Build the RuleItem with explicit field declaration order
    // (id, text, deleted, createdAt) so on-disk JSON matches TS
    // object-literal insertion order.
    let mut rule_obj = Map::new();
    rule_obj.insert("id".to_string(), Value::from(next_id));
    rule_obj.insert("text".to_string(), Value::String(args.rule.clone()));
    rule_obj.insert("deleted".to_string(), Value::Bool(false));
    rule_obj.insert("createdAt".to_string(), Value::String(now.clone()));

    // Append to `rules` (init if missing or non-array).
    let rules_entry = wu
        .extra
        .entry("rules".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !rules_entry.is_array() {
        *rules_entry = Value::Array(Vec::new());
    }
    let rules_len = if let Value::Array(arr) = rules_entry {
        arr.push(Value::Object(rule_obj));
        arr.len()
    } else {
        0
    };

    // Post-increment nextRuleId.
    wu.extra
        .insert("nextRuleId".to_string(), Value::from(next_id + 1));

    // Bump updatedAt.
    wu.updated_at = now;

    // Single atomic write.
    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    let result = AddRuleResult {
        success: true,
        rule_count: rules_len,
    };
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "add-rule",
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
        let a: AddRuleArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","rule":"r1"}"#).unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert_eq!(a.rule, "r1");
    }

    #[test]
    fn args_parse_fails_without_work_unit_id() {
        let err = serde_json::from_str::<AddRuleArgs>(r#"{"rule":"r1"}"#).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("workunitid"),
            "missing-field error must mention workUnitId; got: {msg}"
        );
    }
}
