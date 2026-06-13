//! `add-dependencies` — Rust port of `src/commands/add-dependencies.ts` (RPC-176).
//!
//! Adds multiple dependency relationships to a single work unit in one call.
//! Mirrors the TS implementation which delegates to `add-dependency` for
//! each entry; the Rust port inlines that logic so we can perform a single
//! atomic write at the end (cleaner than the per-call write loop the TS
//! file-manager-transaction pattern uses).
//!
//! ## Semantics (mirrors src/commands/add-dependencies.ts:25-83 +
//! src/commands/add-dependency.ts:61-254)
//!
//! Iteration order: `blocks → blockedBy → dependsOn → relatesTo`. Within
//! each array, original element order is preserved. Per-entry semantics:
//!
//! * **blocks** (bidirectional): source.blocks gains target id;
//!   target.blockedBy gains source id; if target.status != blocked/done it
//!   auto-transitions to `blocked` (states arrays kept in sync).
//! * **blockedBy** (bidirectional): mirror of blocks; if source.status !=
//!   blocked/done it auto-transitions to `blocked` with
//!   `blockedReason = "Blocked by <targetId>"`.
//! * **dependsOn** (unidirectional): only source.dependsOn is mutated.
//! * **relatesTo** (symmetric): both sides gain each other's id with an
//!   idempotent `!contains` guard on the reverse edge.
//!
//! Validation per entry (in order): source-exists, target-exists, no
//! self-dependency, no duplicate, no `blocks`-chain cycle (DFS over the
//! `blocks` adjacency, matching `add-dependency.ts:22-59`).
//!
//! ## Persistence
//!
//! Single `ensure_work_units_file` load + single `write_json_atomic` write
//! at the end. On the first validation error we abort BEFORE writing — no
//! partial state ever lands on disk. This is a deliberate divergence from
//! the TS per-call write loop (which CAN leave partial state if an inner
//! call fails after earlier successes); the end-state of every successful
//! run is identical and the cleaner Rust semantic is preferable for the
//! supervisor-validated dispatcher tests.

use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;
use crate::types::work_unit::{WorkUnit, WorkUnitStatus, WorkUnitsData};

// ─────────────────────────────────────────────────────────────────────────
// Args
// ─────────────────────────────────────────────────────────────────────────

/// CLI arguments accepted by `add-dependencies`. Mirrors the TS
/// `AddDependenciesOptions` shape at `src/commands/add-dependencies.ts:9-18`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct AddDependenciesArgs {
    work_unit_id: String,
    dependencies: DepFlags,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct DepFlags {
    blocks: Option<Vec<String>>,
    blocked_by: Option<Vec<String>>,
    depends_on: Option<Vec<String>>,
    relates_to: Option<Vec<String>>,
}

// ─────────────────────────────────────────────────────────────────────────
// Result
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct AddDependenciesResult {
    success: bool,
    added: usize,
}

// ─────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddDependenciesArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-dependencies",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Load (auto-create on first run).
    let mut data = ensure_work_units_file(project_root)?;

    // Source-exists pre-flight.
    if !data.work_units.contains_key(&args.work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-dependencies",
            reason: format!("Work unit '{}' does not exist", args.work_unit_id),
        });
    }

    let mut added = 0_usize;

    if let Some(ids) = &args.dependencies.blocks {
        for target in ids {
            apply_blocks(&mut data, &args.work_unit_id, target)?;
            added += 1;
        }
    }
    if let Some(ids) = &args.dependencies.blocked_by {
        for target in ids {
            apply_blocked_by(&mut data, &args.work_unit_id, target)?;
            added += 1;
        }
    }
    if let Some(ids) = &args.dependencies.depends_on {
        for target in ids {
            apply_depends_on(&mut data, &args.work_unit_id, target)?;
            added += 1;
        }
    }
    if let Some(ids) = &args.dependencies.relates_to {
        for target in ids {
            apply_relates_to(&mut data, &args.work_unit_id, target)?;
            added += 1;
        }
    }

    // Bump updatedAt on the source unit.
    if let Some(src) = data.work_units.get_mut(&args.work_unit_id) {
        src.updated_at = iso8601_now();
    }

    // Atomic write.
    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    serde_json::to_string(&AddDependenciesResult {
        success: true,
        added,
    })
    .map_err(|e| FspecCoreError::InvalidArgs {
        command: "add-dependencies",
        reason: format!("failed to serialize result: {e}"),
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
    // Cycle detection: follow `blocks` adjacency from target back to source.
    if let Some(cycle) = detect_cycle(data, source_id, target_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-dependencies",
            reason: format!("Circular dependency detected: {source_id} -> {cycle}"),
        });
    }
    // Push target onto source.blocks.
    push_into_list_field(data, source_id, "blocks", target_id);
    // Push source onto target.blockedBy.
    push_into_list_field(data, target_id, "blockedBy", source_id);
    // Auto-transition target to blocked if not blocked/done.
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
    // Cycle detection from the BLOCKER's perspective (TS add-dependency.ts:153-162).
    if let Some(cycle) = detect_cycle(data, target_id, source_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-dependencies",
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
    // Idempotent symmetric reverse edge.
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
            command: "add-dependencies",
            reason: format!("Target work unit '{target_id}' does not exist"),
        });
    }
    Ok(())
}

fn self_dep_error() -> FspecCoreError {
    FspecCoreError::InvalidArgs {
        command: "add-dependencies",
        reason: "Cannot create self-dependency".to_string(),
    }
}

fn duplicate_error() -> FspecCoreError {
    FspecCoreError::InvalidArgs {
        command: "add-dependencies",
        reason: "Dependency already exists".to_string(),
    }
}

/// Read a string-array field from a work unit's `extra` map. Returns an
/// empty Vec when the field is absent or non-array. Mirrors TS `?.length`
/// short-circuit semantics.
fn list_field(wu: &WorkUnit, field: &str) -> Vec<String> {
    match wu.extra.get(field) {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

/// Append `value` to the array-typed `field` on the work unit with id
/// `id`, creating the array if absent. Mirrors `wu.<field> = wu.<field> ||
/// []; wu.<field>.push(value)` from TS.
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

/// Transition `id` from `old_status` to `blocked`, updating both the
/// per-work-unit `status` field AND the `states.<old>` / `states.blocked`
/// arrays. When `blocked_reason` is `Some`, it is stored in the work
/// unit's `extra.blockedReason` field (mirrors the TS `workUnit.blockedReason
/// = '...'` assignment at add-dependency.ts:181).
fn transition_to_blocked(
    data: &mut WorkUnitsData,
    id: &str,
    old_status: WorkUnitStatus,
    blocked_reason: Option<String>,
) {
    // Drop from old state array.
    let old_arr = state_array_mut(&mut data.states, old_status);
    old_arr.retain(|s| s != id);
    // Add to blocked state array (de-dup).
    let blocked_arr = &mut data.states.blocked;
    if !blocked_arr.iter().any(|s| s == id) {
        blocked_arr.push(id.to_string());
    }
    // Update the unit's status + optional blockedReason.
    let Some(wu) = data.work_units.get_mut(id) else {
        return;
    };
    wu.status = WorkUnitStatus::Blocked;
    if let Some(reason) = blocked_reason {
        wu.extra
            .insert("blockedReason".to_string(), Value::String(reason));
    }
}

fn state_array_mut(
    states: &mut crate::types::work_unit::WorkUnitStates,
    s: WorkUnitStatus,
) -> &mut Vec<String> {
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
/// from `to_id` back to `from_id` exists; the returned string is the
/// "->"-joined trail starting at `to_id`. `Some("AUTH-002 -> AUTH-001")`
/// means a cycle exists where `from_id == "AUTH-001"` and the trail of
/// targets is AUTH-002 → AUTH-001.
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
