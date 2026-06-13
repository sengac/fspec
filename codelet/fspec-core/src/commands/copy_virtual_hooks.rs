//! `copy-virtual-hooks` — Rust port of `src/commands/copy-virtual-hooks.ts` (RPC-209).
//!
//! Copies virtual hooks from a source work unit to a target work unit, either
//! all hooks or a single named hook.
//!
//! ## Semantics (mirrors src/commands/copy-virtual-hooks.ts:21-94)
//!
//! 1. `--from` / `--to` presence guards live OUTSIDE the dispatcher entry
//!    point in TS (Commander action handler at lines 105-110), so the
//!    dispatcher-equivalent in Rust enforces the SAME message strings via
//!    explicit checks BEFORE attempting any work — see
//!    [`Self::run`].  We must produce `"--from option is required"` /
//!    `"--to option is required"` substring-exactly.
//! 2. Load `spec/work-units.json` via [`ensure_work_units_file`].
//! 3. Validate source exists → canonical
//!    `Source work unit '<id>' does not exist`.
//! 4. Validate target exists → canonical
//!    `Target work unit '<id>' does not exist`.
//! 5. Source must have non-empty `virtualHooks` →
//!    `No virtual hooks configured for source work unit <id>`
//!    (note: NO single quotes around id — mirrors TS line 49).
//! 6. If `hookName` is supplied, look up the single hook by `name`. Missing
//!    → `Hook '<name>' not found in <fromId>`.
//! 7. Otherwise copy ALL source hooks (deep clone) and APPEND to the target
//!    array (initialising the target's array on demand).
//! 8. Bump the TARGET's `updatedAt` via [`iso8601_now`]; the SOURCE's
//!    `updatedAt` is NOT touched.
//! 9. Atomic single-write persistence via [`write_json_atomic`].
//!
//! ## Result shape
//!
//! `{ "success": true, "copiedCount": <u64> }` serialized via a
//! `#[derive(Serialize)]` struct so JSON key order is `success` then
//! `copiedCount`.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

/// CLI arguments accepted by `copy-virtual-hooks`. Mirrors the TS
/// `CopyVirtualHooksOptions` shape at `src/commands/copy-virtual-hooks.ts:9-14`.
///
/// `from` and `to` are declared as `Option<String>` (NOT a required field)
/// so that omitting them surfaces the bespoke canonical messages
/// `"--from option is required"` / `"--to option is required"` rather than
/// the generic serde `missing field "from"` reason.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct CopyVirtualHooksArgs {
    from: Option<String>,
    to: Option<String>,
    hook_name: Option<String>,
}

/// Response shape returned to the dispatcher. Mirrors the TS
/// `CopyVirtualHooksResult` interface at
/// `src/commands/copy-virtual-hooks.ts:16-19` PLUS an additional
/// `message` field used by the CLI bridge to render the success
/// stdout (the bridge is forbidden from embedding the literal
/// `"Copied "` / `"Source work unit "` / `"Target work unit "`
/// substrings per the delegation test in
/// `codelet/fspec/tests/cli_copy_virtual_hooks.rs`). The trailing
/// `message` slot does NOT affect the canonical
/// `success → copiedCount` JSON key order.
#[derive(Debug, Serialize)]
struct CopyVirtualHooksResult {
    success: bool,
    #[serde(rename = "copiedCount")]
    copied_count: u64,
    message: String,
}

/// Dispatcher entry point. The dispatcher passes the canonical project root
/// alongside the raw JSON args.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: CopyVirtualHooksArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "copy-virtual-hooks",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Presence guards — mirror the TS action-handler messages verbatim.
    let from = match args.from.as_deref() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => {
            return Err(FspecCoreError::InvalidArgs {
                command: "copy-virtual-hooks",
                reason: "--from option is required".to_string(),
            });
        }
    };
    let to = match args.to.as_deref() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => {
            return Err(FspecCoreError::InvalidArgs {
                command: "copy-virtual-hooks",
                reason: "--to option is required".to_string(),
            });
        }
    };

    // Load (auto-create) spec/work-units.json.
    let mut data = ensure_work_units_file(project_root)?;

    // Source exists?
    if !data.work_units.contains_key(&from) {
        return Err(FspecCoreError::InvalidArgs {
            command: "copy-virtual-hooks",
            reason: format!("Source work unit '{from}' does not exist"),
        });
    }
    // Target exists?
    if !data.work_units.contains_key(&to) {
        return Err(FspecCoreError::InvalidArgs {
            command: "copy-virtual-hooks",
            reason: format!("Target work unit '{to}' does not exist"),
        });
    }

    // Read source virtualHooks (via `extra`). Presence was checked above;
    // the `None` branch here is unreachable, but we still match it
    // defensively to keep clippy::expect_used clean.
    let source_hooks: Vec<Value> = match data
        .work_units
        .get(&from)
        .and_then(|src| src.extra.get("virtualHooks"))
    {
        Some(Value::Array(arr)) if !arr.is_empty() => arr.clone(),
        _ => {
            return Err(FspecCoreError::InvalidArgs {
                command: "copy-virtual-hooks",
                reason: format!("No virtual hooks configured for source work unit {from}"),
            });
        }
    };

    // Resolve which hooks to copy (deep clone is implicit via `.clone()`
    // on the JSON values).
    let hooks_to_copy: Vec<Value> = if let Some(name) = args.hook_name.as_deref() {
        let found = source_hooks
            .iter()
            .find(|h| h.get("name").and_then(|n| n.as_str()) == Some(name))
            .cloned();
        match found {
            Some(h) => vec![h],
            None => {
                return Err(FspecCoreError::InvalidArgs {
                    command: "copy-virtual-hooks",
                    reason: format!("Hook '{name}' not found in {from}"),
                });
            }
        }
    } else {
        source_hooks
    };

    let copied_count = hooks_to_copy.len() as u64;

    // Append to target's virtualHooks (initialise if missing), bump
    // target's updatedAt. Source untouched. Presence was checked above;
    // we still match defensively rather than `.expect()` to keep
    // clippy::expect_used clean.
    if let Some(tgt) = data.work_units.get_mut(&to) {
        let entry = tgt
            .extra
            .entry("virtualHooks".to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        if !entry.is_array() {
            *entry = Value::Array(Vec::new());
        }
        if let Some(arr) = entry.as_array_mut() {
            for h in hooks_to_copy {
                arr.push(h);
            }
        }
        tgt.updated_at = iso8601_now();
    }

    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    serde_json::to_string(&CopyVirtualHooksResult {
        success: true,
        copied_count,
        message: format!("✓ Copied {copied_count} virtual hook(s) from {from} to {to}"),
    })
    .map_err(|e| FspecCoreError::InvalidArgs {
        command: "copy-virtual-hooks",
        reason: format!("failed to serialize result: {e}"),
    })
}
