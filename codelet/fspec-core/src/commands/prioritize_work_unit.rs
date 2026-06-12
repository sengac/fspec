//! `prioritize-work-unit` — Rust port of `src/commands/prioritize-work-unit.ts` (RPC-255).
//!
//! Reorders a single work unit within its current Kanban column (the
//! `states.<status>` array matching the work unit's status). Reordering is
//! rejected for `done` work units (done is ordered by completion time) and
//! across columns. Only the one `states.<status>` array is rewritten; the
//! `workUnits` map insertion order and all other fields are preserved
//! verbatim (no `updatedAt` bump — matching the TS source which never
//! touches a work unit's fields here).
//!
//! Reuses existing shared infrastructure:
//! * [`crate::io::ensure::ensure_work_units_file`] — auto-create + load
//!   `spec/work-units.json` (parity with TS `ensureWorkUnitsFile`).
//! * [`crate::io::locked_file::write_json_atomic`] — single atomic write at
//!   the end (the TS implementation uses `fileManager.transaction`).
//!
//! ## Position polymorphism (str | num)
//!
//! The TS `position` field is `'top' | 'bottom' | number`. The dispatcher
//! passes a raw JSON value, so we accept `Option<serde_json::Value>` and
//! interpret:
//!   - string `"top"`    → index 0
//!   - string `"bottom"` → end of column
//!   - JSON number `n`   → 1-based, so index `n - 1` (rejected when `< 1`)
//!   - anything else with `before`/`after` set → relative placement
//!
//! ## Vec::insert clamping
//!
//! JS `Array.prototype.splice(i, 0, x)` inserts at the end when `i` exceeds
//! the array length. Rust's [`Vec::insert`] panics in that case, so the
//! computed index is clamped to `column.len()` to mirror the JS semantics
//! (TS comment: "Allow positions beyond array length").
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge at
//! `codelet/fspec/src/prioritize_work_unit.rs` is argument marshalling only —
//! no domain logic.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::types::work_unit::{WorkUnitStates, WorkUnitStatus};

/// CLI arguments accepted by `prioritize-work-unit`. Mirrors the TS
/// `PrioritizeWorkUnitOptions` interface at
/// `src/commands/prioritize-work-unit.ts:9-15`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct PrioritizeWorkUnitArgs {
    work_unit_id: String,
    /// `'top' | 'bottom' | number` — preserved as a raw JSON value so the
    /// str|num polymorphism round-trips through the dispatcher.
    position: Option<serde_json::Value>,
    before: Option<String>,
    after: Option<String>,
}

#[derive(Debug, Serialize)]
struct PrioritizeWorkUnitResult {
    success: bool,
}

/// Borrow the mutable `states.<status>` vector matching `status`.
fn column_for<'a>(states: &'a mut WorkUnitStates, status: WorkUnitStatus) -> &'a mut Vec<String> {
    match status {
        WorkUnitStatus::Backlog => &mut states.backlog,
        WorkUnitStatus::Specifying => &mut states.specifying,
        WorkUnitStatus::Testing => &mut states.testing,
        WorkUnitStatus::Implementing => &mut states.implementing,
        WorkUnitStatus::Validating => &mut states.validating,
        WorkUnitStatus::Done => &mut states.done,
        WorkUnitStatus::Blocked => &mut states.blocked,
    }
}

fn invalid_args(reason: String) -> FspecCoreError {
    FspecCoreError::InvalidArgs {
        command: "prioritize-work-unit",
        reason,
    }
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: PrioritizeWorkUnitArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "prioritize-work-unit",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Load (auto-create on first run), matching TS `ensureWorkUnitsFile`.
    let mut data = ensure_work_units_file(project_root)?;

    // ── Existence check (TS: prioritize-work-unit.ts:31-33) ─────────────
    if !data.work_units.contains_key(&args.work_unit_id) {
        return Err(invalid_args(format!(
            "Work unit '{}' does not exist",
            args.work_unit_id
        )));
    }

    let current_status = data
        .work_units
        .get(&args.work_unit_id)
        .map(|w| w.status)
        .expect("work unit exists");
    let current_status_str = current_status.as_str();

    // ── Done guard (TS: prioritize-work-unit.ts:38-42) ──────────────────
    if current_status == WorkUnitStatus::Done {
        return Err(invalid_args(
            "Cannot prioritize work units in done column. Done items are ordered by completion \
             time and cannot be manually reordered. Only backlog, specifying, testing, \
             implementing, validating, blocked can be prioritized."
                .to_string(),
        ));
    }

    // ── before/after existence (TS: prioritize-work-unit.ts:45-50) ──────
    if let Some(before) = &args.before {
        if !data.work_units.contains_key(before) {
            return Err(invalid_args(format!("Work unit '{before}' does not exist")));
        }
    }
    if let Some(after) = &args.after {
        if !data.work_units.contains_key(after) {
            return Err(invalid_args(format!("Work unit '{after}' does not exist")));
        }
    }

    // ── Cross-column guard (TS: prioritize-work-unit.ts:53-68) ──────────
    if let Some(before) = &args.before {
        let before_status = data
            .work_units
            .get(before)
            .map(|w| w.status)
            .expect("before existence checked above");
        if before_status != current_status {
            return Err(invalid_args(format!(
                "Cannot prioritize across columns. {} ({}) and {} ({}) are in different columns.",
                args.work_unit_id,
                current_status_str,
                before,
                before_status.as_str()
            )));
        }
    }
    if let Some(after) = &args.after {
        let after_status = data
            .work_units
            .get(after)
            .map(|w| w.status)
            .expect("after existence checked above");
        if after_status != current_status {
            return Err(invalid_args(format!(
                "Cannot prioritize across columns. {} ({}) and {} ({}) are in different columns.",
                args.work_unit_id,
                current_status_str,
                after,
                after_status.as_str()
            )));
        }
    }

    // ── Data integrity (TS: prioritize-work-unit.ts:74-78) ──────────────
    {
        let column = column_for(&mut data.states, current_status);
        if !column.contains(&args.work_unit_id) {
            return Err(invalid_args(format!(
                "Data integrity error: Work unit {} has status '{}' but is not in \
                 states.{} array. Run 'fspec repair-work-units' to fix data corruption.",
                args.work_unit_id, current_status_str, current_status_str
            )));
        }
    }

    // ── Remove from current position (filter out any duplicates) ────────
    // (TS: prioritize-work-unit.ts:81-83)
    let mut column: Vec<String> = column_for(&mut data.states, current_status)
        .iter()
        .filter(|id| *id != &args.work_unit_id)
        .cloned()
        .collect();

    // ── Determine the new index (TS: prioritize-work-unit.ts:86-117) ────
    let mut new_index: usize = 0;
    let mut handled_relative = false;

    if let Some(position) = &args.position {
        if let Some(s) = position.as_str() {
            if s == "top" {
                new_index = 0;
            } else if s == "bottom" {
                new_index = column.len();
            }
            // any other string falls through with new_index = 0 (TS parity:
            // the typeof/=== chain leaves newIndex at its 0 initializer).
        } else if let Some(n) = position.as_i64() {
            // 1-based → 0-based.
            let zero_based = n - 1;
            if zero_based < 0 {
                return Err(invalid_args(format!(
                    "Invalid position: {n}. Position must be >= 1 (1-based index)"
                )));
            }
            new_index = zero_based as usize;
        }
    } else if let Some(before) = &args.before {
        match column.iter().position(|id| id == before) {
            Some(idx) => new_index = idx,
            None => {
                return Err(invalid_args(format!(
                    "Data integrity error: Work unit {} has status '{}' but is not in \
                     states.{} array. Run 'fspec repair-work-units' to fix data corruption.",
                    before, current_status_str, current_status_str
                )));
            }
        }
        handled_relative = true;
    } else if let Some(after) = &args.after {
        match column.iter().position(|id| id == after) {
            Some(idx) => new_index = idx + 1,
            None => {
                return Err(invalid_args(format!(
                    "Data integrity error: Work unit {} has status '{}' but is not in \
                     states.{} array. Run 'fspec repair-work-units' to fix data corruption.",
                    after, current_status_str, current_status_str
                )));
            }
        }
        handled_relative = true;
    }
    let _ = handled_relative;

    // ── Insert at the new position, clamping to len (JS splice parity) ──
    if new_index > column.len() {
        new_index = column.len();
    }
    column.insert(new_index, args.work_unit_id.clone());

    // Write back the reordered column.
    *column_for(&mut data.states, current_status) = column;

    // ── Single atomic write ─────────────────────────────────────────────
    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    let result = PrioritizeWorkUnitResult { success: true };
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "prioritize-work-unit",
        reason: format!("failed to serialize result: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;

    #[test]
    fn args_parse_camel_case_with_string_position() {
        let a: PrioritizeWorkUnitArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","position":"top"}"#).unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert_eq!(a.position.as_ref().and_then(|v| v.as_str()), Some("top"));
    }

    #[test]
    fn args_parse_numeric_position() {
        let a: PrioritizeWorkUnitArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","position":3}"#).unwrap();
        assert_eq!(a.position.as_ref().and_then(|v| v.as_i64()), Some(3));
    }

    #[test]
    fn args_parse_before_after() {
        let a: PrioritizeWorkUnitArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","before":"AUTH-002"}"#).unwrap();
        assert_eq!(a.before.as_deref(), Some("AUTH-002"));
        assert!(a.after.is_none());
    }
}
