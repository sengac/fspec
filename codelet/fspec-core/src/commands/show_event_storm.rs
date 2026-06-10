//! `show-event-storm` — Rust port of `src/commands/show-event-storm.ts` (RPC-303).
//!
//! Returns the active (non-deleted) Event Storm items for a work unit as a
//! pretty-printed JSON array. Mirrors the TS behaviour exactly:
//!
//! * `workUnitId` not found → `Work unit <id> not found`.
//! * No `eventStorm` field OR no `items` array → `Work unit <id> has no Event Storm data`.
//! * Otherwise → JSON array of items where `deleted !== true` (missing
//!   `deleted` field treated as `false`/retained).
//!
//! Both invocation paths (the LLM-facing dispatcher AND the standalone fspec
//! Rust binary's clap subcommand) call this single function — RPC-003 §7/§11
//! two-front-doors invariant.
//!
//! ## TS-parity rules
//!
//! * Read `spec/work-units.json` via [`crate::io::locked_file::read_or_init_json`]
//!   semantics — TS uses `fileManager.readJSON` with the canonical initial
//!   `{version, workUnits, states}` default which is functionally equivalent
//!   to [`ensure_work_units_file`] here. ENOENT → empty store auto-created on
//!   disk; malformed JSON escalates as `ParseJson`.
//! * The result is `JSON.stringify(data, null, 2)` of the array of items,
//!   which preserves declared item field order. We use `serde_json::Value`
//!   to round-trip the raw item objects verbatim, then pretty-print.

use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;

// ─────────────────────────────────────────────────────────────────────────
// Args
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ShowEventStormArgs {
    /// Required: work-unit identifier whose Event Storm to print.
    #[serde(default)]
    work_unit_id: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ShowEventStormArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "show-event-storm",
            reason: format!("failed to parse args: {e}"),
        })?;

    let work_unit_id = args
        .work_unit_id
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "show-event-storm",
            reason: "failed to parse args: missing required field `workUnitId`".to_string(),
        })?;

    // Auto-creates spec/work-units.json on ENOENT, escalates malformed JSON.
    let data = ensure_work_units_file(project_root)?;

    let Some(wu) = data.work_units.get(&work_unit_id) else {
        return Err(FspecCoreError::InvalidArgs {
            command: "show-event-storm",
            reason: format!("Work unit {work_unit_id} not found"),
        });
    };

    // Read the raw `eventStorm` extra field; bail with TS-parity message
    // when missing OR has no `items` array.
    let items = match wu.extra.get("eventStorm") {
        Some(Value::Object(es)) => match es.get("items") {
            Some(Value::Array(arr)) => arr.clone(),
            _ => {
                return Err(FspecCoreError::InvalidArgs {
                    command: "show-event-storm",
                    reason: format!("Work unit {work_unit_id} has no Event Storm data"),
                });
            }
        },
        _ => {
            return Err(FspecCoreError::InvalidArgs {
                command: "show-event-storm",
                reason: format!("Work unit {work_unit_id} has no Event Storm data"),
            });
        }
    };

    // Filter out soft-deleted items (deleted === true). Missing `deleted`
    // field treated as retained, matching TS `.filter(item => !item.deleted)`.
    let active: Vec<Value> = items
        .into_iter()
        .filter(|item| match item.get("deleted") {
            Some(Value::Bool(b)) => !b,
            _ => true,
        })
        .collect();

    let pretty = serde_json::to_string_pretty(&Value::Array(active)).map_err(|e| {
        FspecCoreError::InvalidArgs {
            command: "show-event-storm",
            reason: format!("failed to serialize result: {e}"),
        }
    })?;
    Ok(pretty)
}

// ─────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;

    #[test]
    fn args_parse_with_defaults_has_no_id() {
        let a: ShowEventStormArgs = serde_json::from_str("{}").unwrap();
        assert!(a.work_unit_id.is_none());
    }

    #[test]
    fn args_parse_camel_case_work_unit_id() {
        let a: ShowEventStormArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001"}"#).unwrap();
        assert_eq!(a.work_unit_id.as_deref(), Some("AUTH-001"));
    }
}
