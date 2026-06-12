//! `repair-work-units` — Rust port of `src/commands/repair-work-units.ts` (RPC-284).
//!
//! Fixes data-integrity issues in `spec/work-units.json`:
//!   1. Rebuilds the `states` index from scratch — every work unit is placed
//!      into `states.<its-status>`, iterating `workUnits` in insertion order.
//!      When a work unit was found in a *different* state array, a
//!      `Moved <id> from <oldState> to <status>` repair message is recorded.
//!   2. Repairs bidirectional dependency links — for `blocks` / `blockedBy` /
//!      `relatesTo`, the reverse link is added to the target work unit when
//!      the target exists and the reverse link is missing.
//!
//! Reuses existing shared infrastructure:
//! * [`crate::io::ensure::ensure_work_units_file`] — auto-create + load.
//! * [`crate::io::locked_file::write_json_atomic`] — single atomic write.
//!
//! ## `--dry-run` parity bug (intentional)
//!
//! The TS source declares a `--dry-run` flag but the implementation never
//! reads it — `repairWorkUnits` always writes the rebuilt states to disk.
//! We preserve that exact behaviour: `dryRun` is parsed (so the JSON shape
//! matches) but has NO effect.
//!
//! ## Borrow-checker strategy
//!
//! The bidirectional-link repair both *reads* a source work unit's
//! `blocks`/`blockedBy`/`relatesTo` arrays and *mutates* the target work
//! unit's reverse array. To avoid simultaneous `&mut` borrows over the
//! `IndexMap`, we first collect a mutation plan `(source_id, kind,
//! target_id)` in source-insertion order, then apply each entry one at a
//! time via `get_mut`. Applying sequentially (reading the target fresh each
//! time) reproduces the TS in-place "already linked?" check exactly.
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge at
//! `codelet/fspec/src/repair_work_units.rs` marshals only `{dryRun?}` and
//! prints `✓ Repaired <n> issues`.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::types::work_unit::{WorkUnitStates, WorkUnitStatus};

/// CLI arguments accepted by `repair-work-units`. Mirrors the TS
/// `RepairWorkUnitsOptions` interface at
/// `src/commands/repair-work-units.ts:9-11`. `dryRun` is accepted but
/// IGNORED (TS parity bug — see module docs).
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct RepairWorkUnitsArgs {
    #[allow(dead_code)]
    dry_run: Option<bool>,
}

/// Dispatcher result shape. Returned as JSON from [`run`]. Matches TS
/// `{ success: true, repairs: string[], repaired: number }` at
/// `src/commands/repair-work-units.ts:121-125`.
#[derive(Debug, Serialize)]
struct RepairWorkUnitsResult {
    success: bool,
    repairs: Vec<String>,
    repaired: usize,
}

/// Direction of a dependency edge as read from a source work unit.
#[derive(Debug, Clone, Copy)]
enum LinkKind {
    Blocks,
    BlockedBy,
    RelatesTo,
}

/// The 7 Kanban state arrays in canonical on-disk key order. Used both to
/// scan the OLD states for misplaced ids and to label the `Moved ...` repair
/// messages with the originating state name.
const STATE_NAMES: &[(&str, WorkUnitStatus)] = &[
    ("backlog", WorkUnitStatus::Backlog),
    ("specifying", WorkUnitStatus::Specifying),
    ("testing", WorkUnitStatus::Testing),
    ("implementing", WorkUnitStatus::Implementing),
    ("validating", WorkUnitStatus::Validating),
    ("done", WorkUnitStatus::Done),
    ("blocked", WorkUnitStatus::Blocked),
];

fn old_state_array<'a>(states: &'a WorkUnitStates, status: WorkUnitStatus) -> &'a [String] {
    match status {
        WorkUnitStatus::Backlog => &states.backlog,
        WorkUnitStatus::Specifying => &states.specifying,
        WorkUnitStatus::Testing => &states.testing,
        WorkUnitStatus::Implementing => &states.implementing,
        WorkUnitStatus::Validating => &states.validating,
        WorkUnitStatus::Done => &states.done,
        WorkUnitStatus::Blocked => &states.blocked,
    }
}

fn push_into<'a>(states: &'a mut WorkUnitStates, status: WorkUnitStatus, id: &str) {
    let column = match status {
        WorkUnitStatus::Backlog => &mut states.backlog,
        WorkUnitStatus::Specifying => &mut states.specifying,
        WorkUnitStatus::Testing => &mut states.testing,
        WorkUnitStatus::Implementing => &mut states.implementing,
        WorkUnitStatus::Validating => &mut states.validating,
        WorkUnitStatus::Done => &mut states.done,
        WorkUnitStatus::Blocked => &mut states.blocked,
    };
    column.push(id.to_string());
}

/// Read a string array field from a work unit's `extra` map. Returns an
/// empty vector when the field is absent or not an array of strings.
fn read_string_array(extra: &serde_json::Map<String, Value>, key: &str) -> Vec<String> {
    extra
        .get(key)
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    // Parse args (dryRun is accepted but ignored — TS parity bug).
    let _args: RepairWorkUnitsArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "repair-work-units",
            reason: format!("failed to parse args: {e}"),
        })?;

    let mut data = ensure_work_units_file(project_root)?;

    let mut repairs: Vec<String> = Vec::new();

    // ── Pass 1: rebuild states index from scratch ──────────────────────
    // (TS: repair-work-units.ts:30-57)
    let old_states = data.states.clone();
    let mut new_states = WorkUnitStates {
        backlog: vec![],
        specifying: vec![],
        testing: vec![],
        implementing: vec![],
        validating: vec![],
        done: vec![],
        blocked: vec![],
    };

    // Collect (id, status) in workUnits insertion order first so we don't
    // hold an immutable borrow of `data.work_units` while pushing.
    let placements: Vec<(String, WorkUnitStatus)> = data
        .work_units
        .iter()
        .map(|(id, wu)| (id.clone(), wu.status))
        .collect();

    for (id, status) in &placements {
        push_into(&mut new_states, *status, id);

        // Record a move message for every OLD state array (other than the
        // correct one) that contained this id — iterating states in
        // canonical key order to match TS `Object.entries` ordering.
        for (state_name, state_status) in STATE_NAMES {
            if *state_status == *status {
                continue;
            }
            if old_state_array(&old_states, *state_status)
                .iter()
                .any(|x| x == id)
            {
                repairs.push(format!(
                    "Moved {} from {} to {}",
                    id,
                    state_name,
                    status.as_str()
                ));
            }
        }
    }

    data.states = new_states;

    // ── Pass 2: repair bidirectional dependency links ───────────────────
    // (TS: repair-work-units.ts:59-114)
    //
    // Build the mutation plan in source-insertion order. Within a single
    // work unit the TS source processes blocks, then blockedBy, then
    // relatesTo — so we collect in that order.
    let mut plan: Vec<(String, LinkKind, String)> = Vec::new();
    for (id, wu) in data.work_units.iter() {
        for target in read_string_array(&wu.extra, "blocks") {
            plan.push((id.clone(), LinkKind::Blocks, target));
        }
        for target in read_string_array(&wu.extra, "blockedBy") {
            plan.push((id.clone(), LinkKind::BlockedBy, target));
        }
        for target in read_string_array(&wu.extra, "relatesTo") {
            plan.push((id.clone(), LinkKind::RelatesTo, target));
        }
    }

    for (source_id, kind, target_id) in plan {
        // Skip when the target work unit does not exist (TS guards every
        // branch with `if (workUnitsData.workUnits[targetId])`).
        if !data.work_units.contains_key(&target_id) {
            continue;
        }

        // Resolve the reverse field name + the value that should appear in
        // it, plus the canonical repair message.
        let (reverse_field, reverse_value, message) = match kind {
            LinkKind::Blocks => (
                "blockedBy",
                source_id.clone(),
                format!("Repaired bidirectional link: {source_id} blocks {target_id}"),
            ),
            LinkKind::BlockedBy => (
                "blocks",
                source_id.clone(),
                format!("Repaired bidirectional link: {target_id} blocks {source_id}"),
            ),
            LinkKind::RelatesTo => (
                "relatesTo",
                source_id.clone(),
                format!("Repaired bidirectional link: {source_id} relates to {target_id}"),
            ),
        };

        let target = data
            .work_units
            .get_mut(&target_id)
            .expect("target existence checked above");

        let entry = target
            .extra
            .entry(reverse_field.to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        if !entry.is_array() {
            *entry = Value::Array(Vec::new());
        }
        if let Value::Array(arr) = entry {
            let already = arr.iter().any(|v| v.as_str() == Some(reverse_value.as_str()));
            if !already {
                arr.push(Value::String(reverse_value));
                repairs.push(message);
            }
        }
    }

    // ── Single atomic write (always — TS dryRun is a no-op flag) ────────
    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    let repaired = repairs.len();
    let result = RepairWorkUnitsResult {
        success: true,
        repairs,
        repaired,
    };
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "repair-work-units",
        reason: format!("failed to serialize result: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;

    #[test]
    fn args_parse_empty_object() {
        let a: RepairWorkUnitsArgs = serde_json::from_str("{}").unwrap();
        assert!(a.dry_run.is_none());
    }

    #[test]
    fn args_parse_dry_run_flag() {
        let a: RepairWorkUnitsArgs = serde_json::from_str(r#"{"dryRun":true}"#).unwrap();
        assert_eq!(a.dry_run, Some(true));
    }

    #[test]
    fn read_string_array_handles_missing_and_non_array() {
        let mut m = serde_json::Map::new();
        assert!(read_string_array(&m, "blocks").is_empty());
        m.insert("blocks".to_string(), Value::String("nope".to_string()));
        assert!(read_string_array(&m, "blocks").is_empty());
        m.insert(
            "blocks".to_string(),
            Value::Array(vec![Value::String("AUTH-002".to_string())]),
        );
        assert_eq!(read_string_array(&m, "blocks"), vec!["AUTH-002".to_string()]);
    }
}
