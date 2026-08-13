//! `add-assumption` — Rust port of `src/commands/add-assumption.ts` (RPC-169).
//!
//! Appends a raw assumption string to a work unit's `assumptions` array
//! during the specifying phase. Unlike rules/examples/questions/notes,
//! assumptions are plain strings (no stable-id wrapper) per the TS
//! `WorkUnit.assumptions?: string[]` type at `src/types/index.ts:160`.
//!
//! Reuses existing shared infrastructure:
//! * [`crate::io::ensure::ensure_work_units_file`]
//! * [`crate::io::locked_file::write_json_atomic`]
//! * [`crate::io::time::iso8601_now`]
//!
//! Two-front-doors: bridge marshals JSON `{workUnitId, assumption}` and
//! forwards to this single source-of-truth.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddAssumptionArgs {
    work_unit_id: String,
    assumption: String,
}

#[derive(Debug, Serialize)]
struct AddAssumptionResult {
    success: bool,
    #[serde(rename = "assumptionCount")]
    assumption_count: usize,
}

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddAssumptionArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-assumption",
            reason: format!("failed to parse args: {e}"),
        })?;

    let mut data = ensure_work_units_file(project_root)?;

    // Validate work unit exists.
    let wu = match data.work_units.get_mut(&args.work_unit_id) {
        Some(wu) => wu,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "add-assumption",
                reason: format!("Work unit '{}' does not exist", args.work_unit_id),
            });
        }
    };

    // Validate specifying status.
    let status_str = wu.status.as_str();
    if status_str != "specifying" {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-assumption",
            reason: format!(
                "Can only add assumptions during discovery/specification phase. {} is in '{}' state.",
                args.work_unit_id, status_str
            ),
        });
    }

    let now = iso8601_now();

    // Append the raw assumption string (init array if missing or non-array).
    let assumptions_entry = wu
        .extra
        .entry("assumptions".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !assumptions_entry.is_array() {
        *assumptions_entry = Value::Array(Vec::new());
    }
    let count = if let Value::Array(arr) = assumptions_entry {
        arr.push(Value::String(args.assumption.clone()));
        arr.len()
    } else {
        0
    };

    wu.updated_at = now;

    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    let result = AddAssumptionResult {
        success: true,
        assumption_count: count,
    };
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "add-assumption",
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
        let a: AddAssumptionArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","assumption":"A1"}"#).unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert_eq!(a.assumption, "A1");
    }
}
