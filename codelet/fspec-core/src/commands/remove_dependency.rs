//! `remove-dependency` — Rust port of `src/commands/remove-dependency.ts` (RPC-271).
//!
//! Removes a single dependency relationship from a work unit, with
//! bidirectional cleanup for `blocks`/`blockedBy` and `relatesTo` edges,
//! and unidirectional cleanup for `dependsOn`.
//!
//! ## Semantics (mirrors src/commands/remove-dependency.ts:22-131)
//!
//! Iteration order: `blocks → blockedBy → dependsOn → relatesTo` (matches
//! the TS branch order at lines 37/60/85/97). Each branch performs the
//! same shape of mutation:
//!
//! 1. Filter the named id out of the source unit's array.
//! 2. If the resulting array is empty, **delete** the field entirely
//!    (TS `delete workUnit.<field>`).
//! 3. For bidirectional edges (`blocks`/`blockedBy`/`relatesTo`), perform
//!    the symmetric filter+delete on the target's reverse-edge array.
//! 4. If the target work unit does not exist, the reverse-edge cleanup is
//!    silently skipped — no error, no side-effect (TS guard
//!    `if (data.workUnits[options.blocks])`).
//!
//! ### Critical divergence from `add-*` commands
//!
//! Removal **NEVER**:
//!
//! * changes any work unit's status (a unit blocked-only by the edge
//!   being removed remains `status=blocked` — no auto-revert);
//! * mutates any `states.<status>` array;
//! * performs cycle detection (removing edges cannot create cycles);
//! * touches the target's `updatedAt` (only the source's `updatedAt` is
//!   bumped).
//!
//! ## Persistence
//!
//! Single `ensure_work_units_file` load + single `write_json_atomic` write
//! at the end. The TS file uses `fileManager.transaction()` for the same
//! "load → mutate in-memory → atomic write" lifecycle.
//!
//! ## Args shape (JSON)
//!
//! ```json
//! { "workUnitId": "AUTH-001",
//!   "blocks":     "AUTH-002",   // optional, singular string (NOT array)
//!   "blockedBy":  "API-001",    // optional
//!   "dependsOn":  "AUTH-003",   // optional
//!   "relatesTo":  "AUTH-004"    // optional
//! }
//! ```
//!
//! All-empty relationship args is accepted by the dispatcher as a silent
//! no-op (`success:true`, no mutations). The CLI bridge enforces the
//! at-least-one guard before this entry point ever sees the request —
//! matching where the TS check lives (Commander action handler, not the
//! core function).

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

/// CLI arguments accepted by `remove-dependency`. Mirrors the TS
/// `RemoveDependencyOptions` shape at `src/commands/remove-dependency.ts:9-16`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct RemoveDependencyArgs {
    work_unit_id: String,
    blocks: Option<String>,
    blocked_by: Option<String>,
    depends_on: Option<String>,
    relates_to: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────
// Result
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct RemoveDependencyResult {
    success: bool,
}

// ─────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RemoveDependencyArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "remove-dependency",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Load (auto-create on first run).
    let mut data = ensure_work_units_file(project_root)?;

    // Source-exists pre-flight (TS remove-dependency.ts:30-32).
    if !data.work_units.contains_key(&args.work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "remove-dependency",
            reason: format!("Work unit '{}' does not exist", args.work_unit_id),
        });
    }

    let source_id = args.work_unit_id.clone();

    // ── blocks branch (bidirectional) ──
    if let Some(target_id) = args.blocks.as_deref() {
        filter_field_on_unit(&mut data, &source_id, "blocks", target_id);
        if data.work_units.contains_key(target_id) {
            filter_field_on_unit(&mut data, target_id, "blockedBy", &source_id);
        }
    }

    // ── blockedBy branch (bidirectional) ──
    if let Some(target_id) = args.blocked_by.as_deref() {
        filter_field_on_unit(&mut data, &source_id, "blockedBy", target_id);
        if data.work_units.contains_key(target_id) {
            filter_field_on_unit(&mut data, target_id, "blocks", &source_id);
        }
    }

    // ── dependsOn branch (UNIDIRECTIONAL) ──
    if let Some(target_id) = args.depends_on.as_deref() {
        filter_field_on_unit(&mut data, &source_id, "dependsOn", target_id);
    }

    // ── relatesTo branch (bidirectional symmetric) ──
    if let Some(target_id) = args.relates_to.as_deref() {
        filter_field_on_unit(&mut data, &source_id, "relatesTo", target_id);
        if data.work_units.contains_key(target_id) {
            filter_field_on_unit(&mut data, target_id, "relatesTo", &source_id);
        }
    }

    // Bump updatedAt on the SOURCE unit only (TS line 121).
    if let Some(src) = data.work_units.get_mut(&source_id) {
        src.updated_at = iso8601_now();
    }

    // Single atomic write at end.
    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    serde_json::to_string(&RemoveDependencyResult { success: true }).map_err(|e| {
        FspecCoreError::InvalidArgs {
            command: "remove-dependency",
            reason: format!("failed to serialize result: {e}"),
        }
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

/// Filter `target_value` out of the array-typed `field` on the work unit
/// with id `id`, deleting the field when the resulting array is empty.
/// No-op when the field is absent, non-array, or already lacks the value.
///
/// Mirrors the TS pattern from lines 38-43:
/// ```ts
/// workUnit.blocks = workUnit.blocks.filter(id => id !== options.blocks);
/// if (workUnit.blocks.length === 0) { delete workUnit.blocks; }
/// ```
fn filter_field_on_unit(
    data: &mut WorkUnitsData,
    id: &str,
    field: &str,
    target_value: &str,
) {
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
