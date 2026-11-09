//! The events a global hook runs on (BUG-590).
//!
//! A global hook in spec/fspec-hooks.json runs on `pre-<command>` or
//! `post-<command>` of an fspec command that runs command hooks (HOOK-018,
//! [`is_hooked_command`]), or on `pre-<status>` or `post-<status>` of a
//! workflow status, which update-work-unit-status runs for every work unit
//! (HOOK-020). add-hook writes no hook for any other event, because nothing
//! would ever run it.

use super::phases::is_hooked_command;
use crate::types::work_unit::WorkUnitStatus;

/// The workflow statuses, in board order.
const STATUSES: [WorkUnitStatus; 7] = [
    WorkUnitStatus::Backlog,
    WorkUnitStatus::Specifying,
    WorkUnitStatus::Testing,
    WorkUnitStatus::Implementing,
    WorkUnitStatus::Validating,
    WorkUnitStatus::Done,
    WorkUnitStatus::Blocked,
];

/// Check that `event` is an event a global hook runs on; the exact reason
/// when it is not.
pub fn check_hook_event(event: &str) -> Result<(), String> {
    let target = event
        .strip_prefix("pre-")
        .or_else(|| event.strip_prefix("post-"));
    let runs = target.is_some_and(|target| {
        is_hooked_command(target) || STATUSES.iter().any(|status| status.as_str() == target)
    });
    if runs {
        return Ok(());
    }
    let statuses: Vec<&str> = STATUSES.iter().map(WorkUnitStatus::as_str).collect();
    Err(format!(
        "unknown hook event '{event}': a global hook runs on pre-<command> or post-<command> \
         of an fspec command that runs command hooks, or on pre-<status> or post-<status> of \
         a workflow status ({})",
        statuses.join(", ")
    ))
}
