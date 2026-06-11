//! `remove-virtual-hook` — Rust port of `src/commands/remove-virtual-hook.ts` (RPC-283).
//!
//! Detaches a named virtual hook (and cleans up any associated git-context
//! script file) from a work unit's `virtualHooks` array in
//! `spec/work-units.json`.
//!
//! ## Semantics (mirrors src/commands/remove-virtual-hook.ts:21-77)
//!
//! 1. Resolve work unit by id; missing → `InvalidArgs("Work unit 'X' does not exist")`.
//! 2. Missing OR empty `virtualHooks` → `InvalidArgs("No virtual hooks configured for X")`.
//! 3. Filter ALL entries whose `name == hookName` out of `virtualHooks`
//!    (TS uses `.filter()` — identical semantics, so duplicate names are
//!    all removed in a single call).
//! 4. If length is unchanged after filtering → `InvalidArgs("Virtual hook 'X' not found in Y")`.
//! 5. Best-effort delete `<project_root>/spec/hooks/.virtual/<id>-<hookName>.sh`
//!    — ALL errors (incl. ENOENT) are swallowed silently, matching the TS
//!    try/catch wrapper at `remove-virtual-hook.ts:56-64`.
//! 6. Bump `updatedAt` on the source unit.
//! 7. Single atomic write via `write_json_atomic`.
//!
//! ## Persistence
//!
//! Single `ensure_work_units_file` load + single `write_json_atomic` write
//! at the end. Script cleanup happens BEFORE the work-units write —
//! matching TS source-of-truth order.
//!
//! ## Result shape (JSON)
//!
//! ```json
//! { "success": true, "remainingCount": <new virtualHooks length> }
//! ```
//!
//! `remainingCount` is camelCase — matching the TS `RemoveVirtualHookResult` shape.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

// ─────────────────────────────────────────────────────────────────────────
// Args
// ─────────────────────────────────────────────────────────────────────────

/// CLI arguments accepted by `remove-virtual-hook`. Mirrors the TS
/// `RemoveVirtualHookOptions` shape at
/// `src/commands/remove-virtual-hook.ts:10-14`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct RemoveVirtualHookArgs {
    work_unit_id: String,
    hook_name: String,
}

// ─────────────────────────────────────────────────────────────────────────
// Result
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct RemoveVirtualHookResult {
    success: bool,
    #[serde(rename = "remainingCount")]
    remaining_count: usize,
}

// ─────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: RemoveVirtualHookArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "remove-virtual-hook",
            reason: format!("failed to parse args: {e}"),
        })?;

    if args.work_unit_id.is_empty() {
        return Err(FspecCoreError::InvalidArgs {
            command: "remove-virtual-hook",
            reason: "missing field `workUnitId`".to_string(),
        });
    }

    let mut data = ensure_work_units_file(project_root)?;

    // Source-exists pre-flight (TS remove-virtual-hook.ts:30-32).
    if !data.work_units.contains_key(&args.work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "remove-virtual-hook",
            reason: format!("Work unit '{}' does not exist", args.work_unit_id),
        });
    }

    // Read existing hooks. Missing-or-empty → "No virtual hooks configured".
    // Mirror TS lines 38-40 which check `!virtualHooks || length === 0`.
    let initial_len = match data
        .work_units
        .get(&args.work_unit_id)
        .and_then(|wu| wu.extra.get("virtualHooks"))
    {
        Some(Value::Array(arr)) if !arr.is_empty() => arr.len(),
        _ => {
            return Err(FspecCoreError::InvalidArgs {
                command: "remove-virtual-hook",
                reason: format!(
                    "No virtual hooks configured for {}",
                    args.work_unit_id
                ),
            });
        }
    };

    // Filter the named hook out (TS `filter(hook => hook.name !== hookName)`).
    // Presence and the virtualHooks array were both verified above; we still
    // structure these lookups defensively (no `.expect()` in non-test code).
    let new_len = match data
        .work_units
        .get_mut(&args.work_unit_id)
        .and_then(|wu| wu.extra.get_mut("virtualHooks"))
    {
        Some(Value::Array(arr)) => {
            arr.retain(|h| {
                h.get("name").and_then(Value::as_str) != Some(args.hook_name.as_str())
            });
            arr.len()
        }
        // Guarded above; treat as no-op to preserve initial length.
        _ => initial_len,
    };

    // Not-found guard mirrors TS lines 49-53.
    if new_len == initial_len {
        return Err(FspecCoreError::InvalidArgs {
            command: "remove-virtual-hook",
            reason: format!(
                "Virtual hook '{}' not found in {}",
                args.hook_name, args.work_unit_id
            ),
        });
    }

    // Best-effort cleanup of the associated script file (TS try/catch wrapper
    // around cleanupVirtualHookScript). Errors of any kind are swallowed.
    cleanup_virtual_hook_script(&args.work_unit_id, &args.hook_name, project_root);

    // Bump updatedAt on the source unit (TS line 67).
    if let Some(wu) = data.work_units.get_mut(&args.work_unit_id) {
        wu.updated_at = iso8601_now();
    }

    // Single atomic write at end.
    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    serde_json::to_string(&RemoveVirtualHookResult {
        success: true,
        remaining_count: new_len,
    })
    .map_err(|e| FspecCoreError::InvalidArgs {
        command: "remove-virtual-hook",
        reason: format!("failed to serialize result: {e}"),
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

/// Best-effort delete of `<project_root>/spec/hooks/.virtual/<id>-<name>.sh`.
/// ALL errors (incl. file-not-found) are swallowed silently — mirroring the
/// TS try/catch wrapper at `remove-virtual-hook.ts:56-64` and the catch in
/// `cleanupVirtualHookScript` at `script-generation.ts:115-122`.
fn cleanup_virtual_hook_script(
    work_unit_id: &str,
    hook_name: &str,
    project_root: &Path,
) {
    let script_path = project_root
        .join("spec")
        .join("hooks")
        .join(".virtual")
        .join(format!("{work_unit_id}-{hook_name}.sh"));
    // Intentionally ignore the Result — see TS try/catch wrapper.
    let _ = std::fs::remove_file(&script_path);
}
