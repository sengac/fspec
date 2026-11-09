//! `add-assumption` — Rust port of `src/commands/add-assumption.ts` (RPC-169).
//!
//! Appends an assumption to a work unit's `assumptions` array during the
//! specifying phase. BUG-633: assumptions are stable-id items like rules
//! (`{id, text, deleted, createdAt}` with `nextAssumptionId`); a card's
//! plain-string assumptions are converted first, keeping their indices
//! (see [`super::assumption_items`]).
//!
//! Reuses existing shared infrastructure:
//! * [`crate::io::ensure::ensure_work_units_file`]
//! * [`crate::io::board::write_work_units`]
//! * [`crate::io::time::iso8601_now`]
//!
//! Two-front-doors: bridge marshals JSON `{workUnitId, assumption}` and
//! forwards to this single source-of-truth.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
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

    // BUG-477: hold the spec lock from the first read to the last write.
    let _spec_lock = crate::io::spec_lock::lock_spec_dir(project_root, "add-assumption")?;
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

    // BUG-633: assumptions are stable-id items; the card's string
    // assumptions are converted first.
    let count = {
        let invalid = |reason| FspecCoreError::InvalidArgs {
            command: "add-assumption",
            reason,
        };
        super::assumption_items::push(&args.work_unit_id, &mut wu.extra, &args.assumption, &now)
            .map_err(invalid)?;
        super::assumption_items::active(&args.work_unit_id, wu.extra.get("assumptions"))
            .map_err(invalid)?
            .len()
    };

    wu.updated_at = now;

    crate::io::board::write_work_units(project_root, &data)?;

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
