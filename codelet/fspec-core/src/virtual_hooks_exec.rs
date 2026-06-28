//! Blocking virtual-hook executor (RPC-319).
//!
//! Virtual hooks are work-unit-scoped, ephemeral quality gates that fire on
//! status transitions (e.g. `post-implementing`, `pre-validating`). This
//! module discovers the hooks attached to a work unit, filters them by the
//! transition event, and executes their shell commands SYNCHRONOUSLY via
//! `std::process::Command` (the whole command tree runs under
//! `poll_sync_future`, so NO real async is permitted here).
//!
//! Execution order (parity with the TS hook integration): virtual hooks run
//! BEFORE any global hooks. Blocking-hook failures surface as a
//! [`FspecCoreError`] so the caller aborts the transition; non-blocking
//! failures are swallowed (only their output is relevant).
//!
//! A virtual hook record (stored in `WorkUnit.extra["virtualHooks"]`) has the
//! shape:
//! ```json
//! { "event": "post-implementing", "command": "npm test",
//!   "blocking": true, "gitContext": false, "name": "test" }
//! ```

use std::path::Path;

use serde_json::Value;

use crate::error::FspecCoreError;
use crate::types::work_unit::WorkUnitsData;

/// Run all virtual hooks on `id` that match the `post-<new_status>` event.
///
/// Returns `Ok(())` when no matching hooks exist or all blocking hooks pass.
/// A blocking hook with a non-zero exit code yields an
/// [`FspecCoreError::Message`] whose text is wrapped for agent surfaces.
pub fn run_for_transition(
    project_root: &Path,
    data: &WorkUnitsData,
    id: &str,
    new_status: &str,
) -> Result<(), FspecCoreError> {
    let Some(wu) = data.work_units.get(id) else {
        return Ok(());
    };

    let hooks = match wu.extra.get("virtualHooks").and_then(Value::as_array) {
        Some(h) => h,
        None => return Ok(()),
    };

    let event = format!("post-{new_status}");

    for hook in hooks {
        let hook_event = hook.get("event").and_then(Value::as_str).unwrap_or("");
        if hook_event != event {
            continue;
        }
        let command = hook.get("command").and_then(Value::as_str).unwrap_or("");
        if command.is_empty() {
            continue;
        }
        let blocking = hook
            .get("blocking")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        // todo: gitContext support (run only on changed files via codelet_git)
        let result = run_one(project_root, command);

        match result {
            Ok(true) => {}
            Ok(false) => {
                if blocking {
                    let name = hook.get("name").and_then(Value::as_str).unwrap_or(command);
                    return Err(FspecCoreError::Message(format!(
                        "<system-reminder>Blocking virtual hook '{name}' failed for {id} ({event})</system-reminder>"
                    )));
                }
            }
            Err(e) => {
                if blocking {
                    return Err(e);
                }
            }
        }
    }

    Ok(())
}

/// Execute a single hook command via the system shell, BLOCKING until exit.
/// Returns `Ok(true)` on a zero exit code, `Ok(false)` on a non-zero code.
fn run_one(project_root: &Path, command: &str) -> Result<bool, FspecCoreError> {
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(project_root)
        .status()
        .map_err(|e| FspecCoreError::Message(format!("failed to spawn hook '{command}': {e}")))?;
    Ok(status.success())
}
