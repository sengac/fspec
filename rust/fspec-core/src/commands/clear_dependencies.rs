//! `clear-dependencies` — Rust port of `src/commands/clear-dependencies.ts` (RPC-204).
//!
//! Removes ALL dependency edges (blocks/blockedBy/dependsOn/relatesTo) from a
//! single work unit, with bidirectional reverse-edge cleanup for
//! `blocks`/`blockedBy`/`relatesTo` and unidirectional removal for
//! `dependsOn`.
//!
//! ## Semantics (mirrors src/commands/clear-dependencies.ts:22-99)
//!
//! 1. **--confirm guard** (lines 24-28): if `confirm` is not `true`, abort
//!    with `Must confirm clearing all dependencies with --confirm flag`
//!    BEFORE any file IO occurs.
//! 2. **Source-exists pre-flight** (lines 33-35): abort with `Work unit
//!    '<id>' does not exist` if the source is missing.
//! 3. **Iteration order**: `blocks → blockedBy → dependsOn → relatesTo`.
//! 4. **blocks branch (bidirectional)** (lines 40-52): for each target id
//!    in `workUnit.blocks`, filter source id out of target.blockedBy and
//!    delete the reverse field if empty. Then drop `workUnit.blocks`.
//! 5. **blockedBy branch (bidirectional mirror)** (lines 55-67).
//! 6. **dependsOn branch (UNIDIRECTIONAL)** (lines 70-72): just delete the
//!    field on the source. No reverse-edge cleanup.
//! 7. **relatesTo branch (symmetric bidirectional)** (lines 75-87): for
//!    each target in `workUnit.relatesTo`, filter source id from
//!    target.relatesTo and delete if empty. Then drop the source field.
//! 8. **updatedAt bump** on source only (line 89).
//! 9. Reverse-edge cleanup is **silently skipped** if the target work unit
//!    does not exist (TS guards `data.workUnits[targetId]?.<field>`).
//!
//! ### Critical divergence from `add-*` commands
//!
//! Clearing **NEVER**:
//!
//! * changes any work unit's status (a unit blocked-only by a cleared edge
//!   remains `status=blocked` — no auto-revert);
//! * mutates any `states.<status>` array;
//! * performs cycle detection (removing edges cannot create cycles);
//! * touches any target's `updatedAt` (only the source's `updatedAt` is
//!   bumped).
//!
//! ## Persistence
//!
//! Single `ensure_work_units_file` load + single `write_json_atomic` write
//! at the end. The TS file uses `fileManager.transaction()` for the same
//! "load → mutate in-memory → atomic write" lifecycle.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;
use crate::types::work_unit::WorkUnitsData;

// ─────────────────────────────────────────────────────────────────────────
// Args
// ─────────────────────────────────────────────────────────────────────────

/// CLI arguments accepted by `clear-dependencies`. Mirrors the TS
/// `ClearDependenciesOptions` shape at `src/commands/clear-dependencies.ts:9-13`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ClearDependenciesArgs {
    work_unit_id: String,
    confirm: bool,
}

// ─────────────────────────────────────────────────────────────────────────
// Result
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ClearDependenciesResult {
    success: bool,
}

// ─────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ClearDependenciesArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "clear-dependencies",
            reason: format!("failed to parse args: {e}"),
        })?;

    // --confirm guard (TS lines 24-28) — BEFORE any file IO.
    if !args.confirm {
        return Err(FspecCoreError::InvalidArgs {
            command: "clear-dependencies",
            reason: "Must confirm clearing all dependencies with --confirm flag".to_string(),
        });
    }

    // Load (auto-create on first run).
    let mut data = ensure_work_units_file(project_root)?;

    // Source-exists pre-flight (TS lines 33-35).
    if !data.work_units.contains_key(&args.work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "clear-dependencies",
            reason: format!("Work unit '{}' does not exist", args.work_unit_id),
        });
    }

    let source_id = args.work_unit_id;

    // ── blocks branch (bidirectional) ──
    let blocks_targets = take_list_field(&mut data, &source_id, "blocks");
    for target_id in &blocks_targets {
        filter_field_on_unit(&mut data, target_id, "blockedBy", &source_id);
    }

    // ── blockedBy branch (bidirectional mirror) ──
    let blocked_by_targets = take_list_field(&mut data, &source_id, "blockedBy");
    for target_id in &blocked_by_targets {
        filter_field_on_unit(&mut data, target_id, "blocks", &source_id);
    }

    // ── dependsOn branch (UNIDIRECTIONAL) ──
    let _ = take_list_field(&mut data, &source_id, "dependsOn");

    // ── relatesTo branch (symmetric bidirectional) ──
    let relates_to_targets = take_list_field(&mut data, &source_id, "relatesTo");
    for target_id in &relates_to_targets {
        filter_field_on_unit(&mut data, target_id, "relatesTo", &source_id);
    }

    // Bump updatedAt on the SOURCE unit only (TS line 89).
    if let Some(src) = data.work_units.get_mut(&source_id) {
        src.updated_at = iso8601_now();
    }

    // Single atomic write at end.
    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    serde_json::to_string(&ClearDependenciesResult { success: true }).map_err(|e| {
        FspecCoreError::InvalidArgs {
            command: "clear-dependencies",
            reason: format!("failed to serialize result: {e}"),
        }
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

/// Take (remove) the string-array `field` from the work unit `id` and
/// return its previous contents. Mirrors the TS pattern:
///
/// ```ts
/// const targets = workUnit.blocks;
/// delete workUnit.blocks;
/// ```
///
/// Returns an empty Vec when the field is absent or non-array.
fn take_list_field(data: &mut WorkUnitsData, id: &str, field: &str) -> Vec<String> {
    let Some(wu) = data.work_units.get_mut(id) else {
        return Vec::new();
    };
    let removed = wu.extra.remove(field);
    match removed {
        Some(Value::Array(arr)) => arr
            .into_iter()
            .filter_map(|v| {
                if let Value::String(s) = v {
                    Some(s)
                } else {
                    None
                }
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Filter `target_value` out of the array-typed `field` on the work unit
/// with id `id`, deleting the field when the resulting array is empty.
/// No-op when the field is absent, non-array, or the work unit itself is
/// missing (TS guard `data.workUnits[targetId]?.<field>`).
///
/// Mirrors the TS pattern from lines 42-48:
/// ```ts
/// data.workUnits[targetId].blockedBy = data.workUnits[targetId].blockedBy
///   .filter(id => id !== options.workUnitId);
/// if (data.workUnits[targetId].blockedBy.length === 0) {
///   delete data.workUnits[targetId].blockedBy;
/// }
/// ```
fn filter_field_on_unit(data: &mut WorkUnitsData, id: &str, field: &str, target_value: &str) {
    let Some(wu) = data.work_units.get_mut(id) else {
        return;
    };
    let entry = match wu.extra.get_mut(field) {
        Some(Value::Array(arr)) => arr,
        _ => return,
    };
    entry.retain(|v| v.as_str() != Some(target_value));
    if entry.is_empty() {
        wu.extra.remove(field);
    }
}
