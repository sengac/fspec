//! `add-dependency` — Rust port of `src/commands/add-dependency.ts` (RPC-177).
//!
//! Adds a SINGLE dependency relationship of one or more kinds (blocks /
//! blockedBy / dependsOn / relatesTo) to a work unit. The TypeScript
//! implementation accepts all four flag fields and processes them in
//! lexical order on the same call; the Rust port mirrors that contract
//! so the dispatcher and the CLI bridge share the same args shape.
//!
//! ## Semantics (mirrors src/commands/add-dependency.ts:61-254)
//!
//! Iteration order on a single call: `blocks → blockedBy → dependsOn →
//! relatesTo`. Per-edge semantics:
//!
//! * **blocks** (bidirectional): source.blocks gains target id;
//!   target.blockedBy gains source id; if target.status != blocked/done
//!   it auto-transitions to `blocked` (states arrays kept in sync).
//! * **blockedBy** (bidirectional): mirror of blocks; if source.status !=
//!   blocked/done it auto-transitions to `blocked` with
//!   `blockedReason = "Blocked by <targetId>"`.
//! * **dependsOn** (unidirectional): only source.dependsOn is mutated.
//! * **relatesTo** (symmetric): both sides gain each other's id with an
//!   idempotent `!contains` guard on the reverse edge.
//!
//! Validation per edge (in order): source-exists, target-exists, no
//! self-dependency, no duplicate, no `blocks`-chain cycle (DFS over the
//! `blocks` adjacency, matching `add-dependency.ts:22-59`).
//!
//! ## Persistence
//!
//! Single `ensure_work_units_file` load + single `write_json_atomic` write
//! at the end. On the first validation error we abort BEFORE writing — no
//! partial state ever lands on disk. This matches the Rust port of
//! `add-dependencies` (RPC-176) and improves on the TS per-call write
//! loop, which CAN leave partial state on multi-flag invocations.

use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;
use crate::types::work_unit::{WorkUnit, WorkUnitStates, WorkUnitStatus, WorkUnitsData};

// ─────────────────────────────────────────────────────────────────────────
// Args
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct AddDependencyArgs {
    work_unit_id: String,
    blocks: Option<String>,
    blocked_by: Option<String>,
    depends_on: Option<String>,
    relates_to: Option<String>,
}

#[derive(Debug, Serialize)]
struct AddDependencyResult {
    success: bool,
}

// ─────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddDependencyArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-dependency",
            reason: format!("failed to parse args: {e}"),
        })?;

    let mut data = ensure_work_units_file(project_root)?;

    if !data.work_units.contains_key(&args.work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-dependency",
            reason: format!("Work unit '{}' does not exist", args.work_unit_id),
        });
    }

    if let Some(target) = &args.blocks {
        apply_blocks(&mut data, &args.work_unit_id, target)?;
    }
    if let Some(target) = &args.blocked_by {
        apply_blocked_by(&mut data, &args.work_unit_id, target)?;
    }
    if let Some(target) = &args.depends_on {
        apply_depends_on(&mut data, &args.work_unit_id, target)?;
    }
    if let Some(target) = &args.relates_to {
        apply_relates_to(&mut data, &args.work_unit_id, target)?;
    }

    if let Some(src) = data.work_units.get_mut(&args.work_unit_id) {
        src.updated_at = iso8601_now();
    }

    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    serde_json::to_string(&AddDependencyResult { success: true }).map_err(|e| {
        FspecCoreError::InvalidArgs {
            command: "add-dependency",
            reason: format!("failed to serialize result: {e}"),
        }
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Per-relationship mutators
// ─────────────────────────────────────────────────────────────────────────

fn apply_blocks(
    data: &mut WorkUnitsData,
    source_id: &str,
    target_id: &str,
) -> Result<(), FspecCoreError> {
    validate_target_exists(data, target_id)?;
    if source_id == target_id {
        return Err(self_dep_error());
    }
    let is_dup = data
        .work_units
        .get(source_id)
        .is_some_and(|wu| list_field(wu, "blocks").iter().any(|v| v == target_id));
    if is_dup {
        return Err(duplicate_error());
    }
    if let Some(cycle) = detect_cycle(data, source_id, target_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-dependency",
            reason: format!("Circular dependency detected: {source_id} -> {cycle}"),
        });
    }
    push_into_list_field(data, source_id, "blocks", target_id);
    push_into_list_field(data, target_id, "blockedBy", source_id);
    if let Some(cur_status) = data.work_units.get(target_id).map(|w| w.status) {
        if cur_status != WorkUnitStatus::Blocked && cur_status != WorkUnitStatus::Done {
            transition_to_blocked(data, target_id, cur_status, None);
        }
    }
    Ok(())
}

fn apply_blocked_by(
    data: &mut WorkUnitsData,
    source_id: &str,
    target_id: &str,
) -> Result<(), FspecCoreError> {
    validate_target_exists(data, target_id)?;
    if source_id == target_id {
        return Err(self_dep_error());
    }
    let is_dup = data
        .work_units
        .get(source_id)
        .is_some_and(|wu| list_field(wu, "blockedBy").iter().any(|v| v == target_id));
    if is_dup {
        return Err(duplicate_error());
    }
    if let Some(cycle) = detect_cycle(data, target_id, source_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-dependency",
            reason: format!("Circular dependency detected: {target_id} -> {cycle}"),
        });
    }
    push_into_list_field(data, source_id, "blockedBy", target_id);
    push_into_list_field(data, target_id, "blocks", source_id);
    if let Some(cur_status) = data.work_units.get(source_id).map(|w| w.status) {
        if cur_status != WorkUnitStatus::Blocked && cur_status != WorkUnitStatus::Done {
            transition_to_blocked(
                data,
                source_id,
                cur_status,
                Some(format!("Blocked by {target_id}")),
            );
        }
    }
    Ok(())
}

fn apply_depends_on(
    data: &mut WorkUnitsData,
    source_id: &str,
    target_id: &str,
) -> Result<(), FspecCoreError> {
    validate_target_exists(data, target_id)?;
    if source_id == target_id {
        return Err(self_dep_error());
    }
    let is_dup = data
        .work_units
        .get(source_id)
        .is_some_and(|wu| list_field(wu, "dependsOn").iter().any(|v| v == target_id));
    if is_dup {
        return Err(duplicate_error());
    }
    push_into_list_field(data, source_id, "dependsOn", target_id);
    Ok(())
}

fn apply_relates_to(
    data: &mut WorkUnitsData,
    source_id: &str,
    target_id: &str,
) -> Result<(), FspecCoreError> {
    validate_target_exists(data, target_id)?;
    if source_id == target_id {
        return Err(self_dep_error());
    }
    let is_dup = data
        .work_units
        .get(source_id)
        .is_some_and(|wu| list_field(wu, "relatesTo").iter().any(|v| v == target_id));
    if is_dup {
        return Err(duplicate_error());
    }
    push_into_list_field(data, source_id, "relatesTo", target_id);
    let reverse_has = data
        .work_units
        .get(target_id)
        .is_some_and(|wu| list_field(wu, "relatesTo").iter().any(|v| v == source_id));
    if !reverse_has {
        push_into_list_field(data, target_id, "relatesTo", source_id);
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn validate_target_exists(data: &WorkUnitsData, target_id: &str) -> Result<(), FspecCoreError> {
    if !data.work_units.contains_key(target_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-dependency",
            reason: format!("Target work unit '{target_id}' does not exist"),
        });
    }
    Ok(())
}

fn self_dep_error() -> FspecCoreError {
    FspecCoreError::InvalidArgs {
        command: "add-dependency",
        reason: "Cannot create self-dependency".to_string(),
    }
}

fn duplicate_error() -> FspecCoreError {
    FspecCoreError::InvalidArgs {
        command: "add-dependency",
        reason: "Dependency already exists".to_string(),
    }
}

fn list_field(wu: &WorkUnit, field: &str) -> Vec<String> {
    match wu.extra.get(field) {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

fn push_into_list_field(data: &mut WorkUnitsData, id: &str, field: &str, value: &str) {
    let Some(wu) = data.work_units.get_mut(id) else {
        return;
    };
    let entry = wu
        .extra
        .entry(field.to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !entry.is_array() {
        *entry = Value::Array(Vec::new());
    }
    if let Value::Array(arr) = entry {
        arr.push(Value::String(value.to_string()));
    }
}

fn transition_to_blocked(
    data: &mut WorkUnitsData,
    id: &str,
    old_status: WorkUnitStatus,
    blocked_reason: Option<String>,
) {
    let old_arr = state_array_mut(&mut data.states, old_status);
    old_arr.retain(|s| s != id);
    let blocked_arr = &mut data.states.blocked;
    if !blocked_arr.iter().any(|s| s == id) {
        blocked_arr.push(id.to_string());
    }
    let Some(wu) = data.work_units.get_mut(id) else {
        return;
    };
    wu.status = WorkUnitStatus::Blocked;
    if let Some(reason) = blocked_reason {
        wu.extra
            .insert("blockedReason".to_string(), Value::String(reason));
    }
}

fn state_array_mut(states: &mut WorkUnitStates, s: WorkUnitStatus) -> &mut Vec<String> {
    match s {
        WorkUnitStatus::Backlog => &mut states.backlog,
        WorkUnitStatus::Specifying => &mut states.specifying,
        WorkUnitStatus::Testing => &mut states.testing,
        WorkUnitStatus::Implementing => &mut states.implementing,
        WorkUnitStatus::Validating => &mut states.validating,
        WorkUnitStatus::Done => &mut states.done,
        WorkUnitStatus::Blocked => &mut states.blocked,
    }
}

/// DFS over the `blocks` adjacency, mirroring `detectCircularDependency` in
/// `src/commands/add-dependency.ts:22-59`. Returns `Some(path)` when a path
/// from `to_id` back to `from_id` exists.
fn detect_cycle(data: &WorkUnitsData, from_id: &str, to_id: &str) -> Option<String> {
    fn dfs(
        units: &IndexMap<String, WorkUnit>,
        from_id: &str,
        cur_id: &str,
        visited: &mut std::collections::HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<String> {
        if visited.contains(cur_id) {
            return None;
        }
        visited.insert(cur_id.to_string());
        path.push(cur_id.to_string());
        if cur_id == from_id && path.len() > 1 {
            return Some(path.join(" -> "));
        }
        if let Some(wu) = units.get(cur_id) {
            for entry in list_field(wu, "blocks") {
                let mut branch_visited = visited.clone();
                let mut branch_path = path.clone();
                if let Some(cycle) = dfs(
                    units,
                    from_id,
                    &entry,
                    &mut branch_visited,
                    &mut branch_path,
                ) {
                    return Some(cycle);
                }
            }
        }
        None
    }

    let mut visited = std::collections::HashSet::new();
    let mut path = Vec::new();
    dfs(&data.work_units, from_id, to_id, &mut visited, &mut path)
}
