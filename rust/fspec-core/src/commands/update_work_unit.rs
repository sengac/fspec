//! `update-work-unit` — Rust port of `src/commands/update-work-unit.ts` (RPC-317).
//!
//! Updates work-unit metadata (title, description, epic, parent) in
//! `spec/work-units.json`, and — when `epic`/`parent` change — keeps the
//! cross-references consistent (epic `workUnits` arrays in `spec/epics.json`,
//! parent `children` arrays in `spec/work-units.json`).
//!
//! Both invocation paths (the LLM-facing dispatcher AND the standalone fspec
//! Rust binary's clap subcommand) call this single function — RPC-003 §7/§11
//! two-front-doors invariant.
//!
//! ## Parity behaviours (vs TS `updateWorkUnit`)
//!
//! - Missing work unit → error `Work unit '<id>' does not exist`
//!   (`update-work-unit.ts:33-35`). NOT wrapped (TS throws raw).
//! - `type` is immutable: any non-undefined `type` arg → error
//!   `Work unit type is immutable and cannot be changed after creation...`
//!   with the full multi-line body (`update-work-unit.ts:38-45`).
//! - `parent`:
//!   - must exist → `Parent work unit '<id>' does not exist`
//!     (`update-work-unit.ts:49-51`).
//!   - circular check (self-parent or ancestor cycle) →
//!     `Circular parent relationship detected` (`update-work-unit.ts:54-62`).
//! - `epic` must exist → `Epic '<id>' does not exist`
//!   (`update-work-unit.ts:66-72`).
//! - Field updates mirror TS exactly: title/description set on the unit;
//!   epic move removes the id from the OLD epic's `workUnits` and adds to the
//!   NEW epic's `workUnits`; parent set removes the id from the OLD parent's
//!   `children` and adds to the NEW parent's `children`.
//! - `updatedAt` is ALWAYS bumped (`update-work-unit.ts:138-139`).
//!
//! ## Field-order / data preservation
//!
//! We round-trip `spec/work-units.json` (and `spec/epics.json`) as raw
//! `serde_json::Map` objects so every existing record keeps its exact on-disk
//! key order and any unmodelled fields survive — matching TS `JSON.parse`/
//! `JSON.stringify`. The TS source mutates the WHOLE in-memory object and then
//! `Object.assign(data, workUnitsData)` writes it back; we reproduce that by
//! mutating the raw object tree in place.
//!
//! Note: the TS implementation writes epics via a SEPARATE
//! `fileManager.transaction(epicsFile, ...)` BEFORE the work-units write, and
//! the validations all run first — so a validation failure never touches
//! disk. We preserve that ordering.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::{ensure_epics_file, ensure_work_units_file};
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

/// CLI/dispatcher arguments accepted by `update-work-unit`. Mirrors the TS
/// `UpdateWorkUnitOptions` shape at `src/commands/update-work-unit.ts:9-17`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct UpdateWorkUnitArgs {
    /// Work unit ID to update. Required in practice; defaulted to empty so a
    /// malformed/empty call surfaces the same "does not exist" path as TS.
    #[serde(default)]
    work_unit_id: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    epic: Option<String>,
    #[serde(default)]
    parent: Option<String>,
    /// Present ONLY to detect the immutable-type-change attempt. The CLI
    /// surface does NOT expose `--type` (parity with TS Commander.js, which
    /// omits it), but the dispatcher arg shape carries it so the immutability
    /// guard fires for LLM callers.
    #[serde(default)]
    r#type: Option<String>,
}

/// Dispatcher result shape. Returned as pretty-printed JSON from [`run`].
/// Matches TS `{ success: true }` at `src/commands/update-work-unit.ts:146`.
#[derive(Debug, Serialize)]
struct UpdateWorkUnitResult {
    success: bool,
}

/// Dispatcher entry point. Both front doors converge here.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: UpdateWorkUnitArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "update-work-unit",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Load typed work units (auto-creates spec/work-units.json on ENOENT,
    // parity with TS `ensureWorkUnitsFile`). Used for existence + circular
    // checks; the actual mutation happens on the raw object tree below.
    let work_units_data = ensure_work_units_file(project_root)?;

    // ── Existence check (TS: update-work-unit.ts:33-35) ────────────────
    if !work_units_data.work_units.contains_key(&args.work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "update-work-unit",
            reason: format!("Work unit '{}' does not exist", args.work_unit_id),
        });
    }

    // ── Type immutability (TS: update-work-unit.ts:38-45) ──────────────
    // Any non-undefined `type` arg is rejected, even if equal to the current
    // type — TS only checks `options.type !== undefined`.
    if let Some(attempted) = args.r#type.as_deref() {
        let current = work_units_data
            .work_units
            .get(&args.work_unit_id)
            .map(crate::types::work_unit::WorkUnit::type_str)
            .unwrap_or("story");
        let reason = format!(
            "Work unit type is immutable and cannot be changed after creation.\n\n\
             Current type: {current}\n\
             Attempted to change to: {attempted}\n\n\
             If you need to change the type, Delete this work unit and create a new one with the correct type."
        );
        return Err(FspecCoreError::InvalidArgs {
            command: "update-work-unit",
            reason,
        });
    }

    // ── Parent validation (TS: update-work-unit.ts:47-63) ──────────────
    // Mirror TS `if (options.parent)` — JS-truthiness, so an empty-string
    // parent is treated as ABSENT (no validation, no mutation).
    let parent_provided = args.parent.as_deref().filter(|p| !p.is_empty());
    if let Some(parent) = parent_provided {
        if !work_units_data.work_units.contains_key(parent) {
            return Err(FspecCoreError::InvalidArgs {
                command: "update-work-unit",
                reason: format!("Parent work unit '{parent}' does not exist"),
            });
        }
        if would_create_circular_reference(
            &work_units_data,
            &args.work_unit_id,
            parent,
            &mut std::collections::HashSet::new(),
        ) {
            return Err(FspecCoreError::InvalidArgs {
                command: "update-work-unit",
                reason: "Circular parent relationship detected".to_string(),
            });
        }
    }

    // ── Epic validation (TS: update-work-unit.ts:65-72) ────────────────
    // Mirror TS `if (options.epic !== undefined)` — `ensureEpicsFile` then
    // existence check. Note TS uses `!== undefined`, so an empty-string epic
    // WOULD be validated; but `epicsData.epics['']` is falsy → "does not
    // exist". We replicate via contains_key on the raw arg.
    if let Some(epic) = args.epic.as_deref() {
        let epics_data = ensure_epics_file(project_root)?;
        if !epics_data.epics.contains_key(epic) {
            return Err(FspecCoreError::InvalidArgs {
                command: "update-work-unit",
                reason: format!("Epic '{epic}' does not exist"),
            });
        }
    }

    // ── Mutation phase ─────────────────────────────────────────────────
    // Round-trip the raw work-units object so existing entries keep their
    // exact key order. Fall back to the typed data on any read failure.
    let mut wu_top: Map<String, Value> =
        read_raw_object(&project_root.join("spec").join("work-units.json")).unwrap_or_else(|| {
            serde_json::to_value(&work_units_data)
                .ok()
                .and_then(|v| match v {
                    Value::Object(m) => Some(m),
                    _ => None,
                })
                .unwrap_or_default()
        });

    // Old epic (for the move) — read from the typed data BEFORE mutation.
    let old_epic = work_units_data
        .work_units
        .get(&args.work_unit_id)
        .and_then(|wu| wu.epic.clone());

    // Old parent (for the children move) — read from typed data.
    let old_parent = work_units_data
        .work_units
        .get(&args.work_unit_id)
        .and_then(|wu| wu.extra.get("parent"))
        .and_then(Value::as_str)
        .map(str::to_string);

    // Apply scalar field updates on the target unit within the raw tree.
    if let Some(unit) = wu_top
        .get_mut("workUnits")
        .and_then(Value::as_object_mut)
        .and_then(|m| m.get_mut(&args.work_unit_id))
        .and_then(Value::as_object_mut)
    {
        if let Some(title) = args.title.as_deref() {
            unit.insert("title".to_string(), Value::String(title.to_string()));
        }
        if let Some(desc) = args.description.as_deref() {
            unit.insert("description".to_string(), Value::String(desc.to_string()));
        }
        if let Some(epic) = args.epic.as_deref() {
            unit.insert("epic".to_string(), Value::String(epic.to_string()));
        }
    }

    // Parent move (TS: update-work-unit.ts:110-135). Done on the work-units
    // raw tree: remove from old parent's children, set parent, add to new
    // parent's children.
    if let Some(parent) = parent_provided {
        // Remove from old parent's children. TS (update-work-unit.ts:112-119)
        // has NO `oldParent !== parent` guard: when the work unit is
        // re-assigned to the SAME parent it is still removed-then-re-appended,
        // moving its id to the END of the children array. Reproduce that
        // bug-for-bug (a guard here would leave the array unreordered).
        if let Some(old) = old_parent.as_deref() {
            if let Some(old_obj) = wu_top
                .get_mut("workUnits")
                .and_then(Value::as_object_mut)
                .and_then(|m| m.get_mut(old))
                .and_then(Value::as_object_mut)
            {
                if let Some(Value::Array(children)) = old_obj.get_mut("children") {
                    children.retain(|c| c.as_str() != Some(args.work_unit_id.as_str()));
                }
            }
        }
        // Set new parent on the unit.
        if let Some(unit) = wu_top
            .get_mut("workUnits")
            .and_then(Value::as_object_mut)
            .and_then(|m| m.get_mut(&args.work_unit_id))
            .and_then(Value::as_object_mut)
        {
            unit.insert("parent".to_string(), Value::String(parent.to_string()));
        }
        // Add to new parent's children.
        if let Some(parent_obj) = wu_top
            .get_mut("workUnits")
            .and_then(Value::as_object_mut)
            .and_then(|m| m.get_mut(parent))
            .and_then(Value::as_object_mut)
        {
            let children = parent_obj
                .entry("children".to_string())
                .or_insert_with(|| Value::Array(Vec::new()));
            if !children.is_array() {
                *children = Value::Array(Vec::new());
            }
            if let Value::Array(arr) = children {
                if !arr
                    .iter()
                    .any(|c| c.as_str() == Some(args.work_unit_id.as_str()))
                {
                    arr.push(Value::String(args.work_unit_id.clone()));
                }
            }
        }
    }

    // Bump updatedAt (TS: update-work-unit.ts:138-139). Always.
    if let Some(unit) = wu_top
        .get_mut("workUnits")
        .and_then(Value::as_object_mut)
        .and_then(|m| m.get_mut(&args.work_unit_id))
        .and_then(Value::as_object_mut)
    {
        unit.insert("updatedAt".to_string(), Value::String(iso8601_now()));
    }

    // Epic move (TS: update-work-unit.ts:84-108). Writes the SEPARATE
    // epics.json file. Done BEFORE the work-units write, matching TS order
    // (the epic transaction runs inside the field-update block, before the
    // final work-units transaction).
    if let Some(new_epic) = args.epic.as_deref() {
        let mut epics_top: Map<String, Value> =
            read_raw_object(&project_root.join("spec").join("epics.json")).unwrap_or_default();
        if let Some(epics_obj) = epics_top.get_mut("epics").and_then(Value::as_object_mut) {
            // Remove from old epic. TS (update-work-unit.ts:94-98) has NO
            // `oldEpic !== epic` guard: a same-epic re-assignment still
            // removes-then-re-appends, moving the id to the END of the epic's
            // workUnits array. Reproduce that bug-for-bug.
            if let Some(old) = old_epic.as_deref() {
                if let Some(old_obj) = epics_obj.get_mut(old).and_then(Value::as_object_mut) {
                    if let Some(Value::Array(units)) = old_obj.get_mut("workUnits") {
                        units.retain(|u| u.as_str() != Some(args.work_unit_id.as_str()));
                    }
                }
            }
            // Add to new epic.
            if let Some(new_obj) = epics_obj.get_mut(new_epic).and_then(Value::as_object_mut) {
                let units = new_obj
                    .entry("workUnits".to_string())
                    .or_insert_with(|| Value::Array(Vec::new()));
                if !units.is_array() {
                    *units = Value::Array(Vec::new());
                }
                if let Value::Array(arr) = units {
                    if !arr
                        .iter()
                        .any(|u| u.as_str() == Some(args.work_unit_id.as_str()))
                    {
                        arr.push(Value::String(args.work_unit_id.clone()));
                    }
                }
            }
        }
        let epics_path = project_root.join("spec").join("epics.json");
        write_json_atomic(&epics_path, &Value::Object(epics_top))?;
    }

    // Atomic write of work-units.json.
    let wu_path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&wu_path, &Value::Object(wu_top))?;

    let result = UpdateWorkUnitResult { success: true };
    serde_json::to_string_pretty(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "update-work-unit",
        reason: format!("failed to serialize result: {e}"),
    })
}

/// Detect whether making `proposed_parent_id` the parent of `work_unit_id`
/// would create a cycle. Verbatim port of `wouldCreateCircularReference`
/// (`src/commands/update-work-unit.ts:149-184`).
fn would_create_circular_reference(
    data: &crate::types::work_unit::WorkUnitsData,
    work_unit_id: &str,
    proposed_parent_id: &str,
    visited: &mut std::collections::HashSet<String>,
) -> bool {
    // Trying to make a work unit its own ancestor is circular.
    if proposed_parent_id == work_unit_id {
        return true;
    }
    // Already visited → cycle.
    if visited.contains(proposed_parent_id) {
        return true;
    }
    visited.insert(proposed_parent_id.to_string());

    let proposed_parent = match data.work_units.get(proposed_parent_id) {
        Some(wu) => wu,
        None => return false,
    };
    // If the proposed parent itself has a parent, recurse up the chain.
    if let Some(grandparent) = proposed_parent.extra.get("parent").and_then(Value::as_str) {
        return would_create_circular_reference(data, work_unit_id, grandparent, visited);
    }
    false
}

/// Read a JSON file as a raw object, preserving key order. Returns `None` on
/// any I/O or parse failure so callers can fall back.
fn read_raw_object(path: &Path) -> Option<Map<String, Value>> {
    let raw = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str::<Value>(&raw).ok()? {
        Value::Object(m) => Some(m),
        _ => None,
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
    fn args_parse_camel_case_fields() {
        let a: UpdateWorkUnitArgs = serde_json::from_str(
            r#"{"workUnitId":"AUTH-001","title":"T","description":"D","epic":"e","parent":"P"}"#,
        )
        .unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert_eq!(a.title.as_deref(), Some("T"));
        assert_eq!(a.description.as_deref(), Some("D"));
        assert_eq!(a.epic.as_deref(), Some("e"));
        assert_eq!(a.parent.as_deref(), Some("P"));
        assert!(a.r#type.is_none());
    }

    #[test]
    fn args_parse_empty_object_yields_empty_id() {
        let a: UpdateWorkUnitArgs = serde_json::from_str("{}").unwrap();
        assert_eq!(a.work_unit_id, "");
        assert!(a.title.is_none());
    }

    #[test]
    fn circular_reference_detects_self_parent() {
        let data = crate::types::work_unit::WorkUnitsData::initial("x");
        let mut visited = std::collections::HashSet::new();
        assert!(would_create_circular_reference(
            &data,
            "AUTH-001",
            "AUTH-001",
            &mut visited
        ));
    }

    #[test]
    fn circular_reference_allows_unrelated_parent() {
        let raw = r#"{
            "workUnits": {
                "A": { "id": "A", "title": "a", "status": "backlog",
                       "createdAt": "x", "updatedAt": "x" },
                "B": { "id": "B", "title": "b", "status": "backlog",
                       "createdAt": "x", "updatedAt": "x" }
            },
            "states": { "backlog": ["A","B"], "specifying": [], "testing": [],
                "implementing": [], "validating": [], "done": [], "blocked": [] }
        }"#;
        let data: crate::types::work_unit::WorkUnitsData = serde_json::from_str(raw).unwrap();
        let mut visited = std::collections::HashSet::new();
        assert!(!would_create_circular_reference(
            &data,
            "A",
            "B",
            &mut visited
        ));
    }
}
