//! `clear-virtual-hooks` — Rust port of `src/commands/clear-virtual-hooks.ts` (RPC-205).
//!
//! Wipes all work-unit-scoped virtual hooks from a single work unit and
//! best-effort deletes their generated script files at
//! `spec/hooks/.virtual/<workUnitId>-<hookName>.sh`.
//!
//! ## Semantics (mirrors src/commands/clear-virtual-hooks.ts:20-69)
//!
//! 1. Load `spec/work-units.json` via [`ensure_work_units_file`] (auto-create
//!    on ENOENT, parity with `ensureWorkUnitsFile`).
//! 2. Look up `data.workUnits[workUnitId]` — missing → `InvalidArgs` with
//!    the canonical TS substring `Work unit '<id>' does not exist`.
//! 3. Capture `clearedCount = workUnit.virtualHooks?.length || 0`. We read
//!    through `wu.extra["virtualHooks"]` since the typed [`WorkUnit`] struct
//!    does NOT expose `virtualHooks` directly (parity with list-virtual-hooks).
//! 4. For each hook, best-effort `std::fs::remove_file(...)` — any error
//!    (ENOENT or otherwise) is silently swallowed, mirroring the bare TS
//!    `try { cleanupVirtualHookScript(...) } catch {}` block.
//! 5. Replace `virtualHooks` with an empty array (`[]`), NOT remove the
//!    field. TS line 55: `workUnit.virtualHooks = []`.
//! 6. Bump the SOURCE unit's `updatedAt` via [`iso8601_now`].
//! 7. Atomic single-write persistence via [`write_json_atomic`] —
//!    `fileManager.transaction` parity.
//!
//! ## Result shape
//!
//! `{ "success": true, "clearedCount": <u64> }` serialized via a `#[derive(Serialize)]`
//! struct so JSON key order is `success` then `clearedCount` (asserted by
//! `scenario_result_json_shape_preserves_field_order`).

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

/// CLI arguments accepted by `clear-virtual-hooks`. Mirrors the TS
/// `ClearVirtualHooksOptions` shape at `src/commands/clear-virtual-hooks.ts:10-13`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClearVirtualHooksArgs {
    /// Work unit ID to clear hooks from. Required.
    work_unit_id: String,
}

/// Response shape returned to the dispatcher. Mirrors the TS
/// `ClearVirtualHooksResult` interface at
/// `src/commands/clear-virtual-hooks.ts:15-18` PLUS an additional
/// `message` field used by the CLI bridge to render the success
/// stdout (the bridge is forbidden from embedding the literal
/// `"Cleared "` substring per the delegation test in
/// `rust/fspec/tests/cli_clear_virtual_hooks.rs`). The trailing
/// `message` slot does NOT affect the canonical
/// `success → clearedCount` JSON key order asserted by
/// `scenario_result_json_shape_preserves_field_order`.
#[derive(Debug, Serialize)]
struct ClearVirtualHooksResult {
    success: bool,
    #[serde(rename = "clearedCount")]
    cleared_count: u64,
    message: String,
}

/// Dispatcher entry point. The dispatcher passes the canonical project root
/// alongside the raw JSON args; we never call `std::env::current_dir()` so
/// the same binary can serve multiple sessions / working-directories safely.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ClearVirtualHooksArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "clear-virtual-hooks",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Load (auto-create) spec/work-units.json — parity with the TS command
    // which calls `ensureWorkUnitsFile(cwd)` unconditionally.
    let mut data = ensure_work_units_file(project_root)?;

    // Validate that the requested work unit exists. We mirror the TS
    // error string `Work unit '<id>' does not exist` (single-quoted id)
    // so dispatcher callers can substring-match.
    if !data.work_units.contains_key(&args.work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "clear-virtual-hooks",
            reason: format!("Work unit '{}' does not exist", args.work_unit_id),
        });
    }

    // Collect hook names BEFORE mutating, so we can drive script cleanup
    // even after we replace virtualHooks with []. Read through `extra`
    // mirroring list-virtual-hooks. Presence was checked above, so the
    // `None` branches here are unreachable — we treat them as empty rather
    // than panic via `.expect()` so clippy::expect_used stays clean.
    let hook_names: Vec<String> = match data
        .work_units
        .get(&args.work_unit_id)
        .and_then(|wu| wu.extra.get("virtualHooks"))
    {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.get("name").and_then(|n| n.as_str()).map(String::from))
            .collect(),
        _ => Vec::new(),
    };

    let cleared_count = hook_names.len() as u64;

    // Best-effort script cleanup. Mirrors the TS try/catch around
    // `cleanupVirtualHookScript`; any error (ENOENT or otherwise) is
    // silently swallowed.
    let virtual_dir = project_root.join("spec/hooks/.virtual");
    for hook_name in &hook_names {
        let script_path = virtual_dir.join(format!("{}-{hook_name}.sh", args.work_unit_id));
        let _ = std::fs::remove_file(&script_path);
    }

    // Mutate: set virtualHooks = [] (NOT remove), bump updatedAt.
    if let Some(wu) = data.work_units.get_mut(&args.work_unit_id) {
        wu.extra
            .insert("virtualHooks".to_string(), Value::Array(Vec::new()));
        wu.updated_at = iso8601_now();
    }

    // Single atomic write at end.
    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    serde_json::to_string(&ClearVirtualHooksResult {
        success: true,
        cleared_count,
        message: format!(
            "✓ Cleared {cleared_count} virtual hook(s) from {}",
            args.work_unit_id
        ),
    })
    .map_err(|e| FspecCoreError::InvalidArgs {
        command: "clear-virtual-hooks",
        reason: format!("failed to serialize result: {e}"),
    })
}
