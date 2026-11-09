//! `update-work-unit-status` — Rust port of `src/commands/update-work-unit-status.ts`.
//!
//! Enforces the ACDD lifecycle. Runs under `poll_sync_future` (the future is
//! polled exactly once), so ALL I/O uses BLOCKING std APIs.

use std::path::Path;

use serde::Deserialize;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::time::iso8601_now;

// BUG-285: `sync_version` reuses the configure-tools checks.
pub(crate) mod reminders;

mod audit_gate;
mod board_state;
mod checkpoints;
mod output;
mod prefill;
mod queries;
mod review;
mod temporal;
mod temporal_mapped;
mod transitions;

#[cfg(test)]
mod tests;

use crate::commands::audit::Area;
use audit_gate::check_audit;
use board_state::{apply_status_change, compact_done, move_state};
use checkpoints::{cleanup_done_checkpoints, maybe_auto_checkpoint};
use output::{render_output, Transition};
use prefill::check_prefill;
use queries::{array_len, incomplete_children};
use review::check_testing_entry;
use temporal::check_temporal_ordering;
use transitions::{check_transition_gates, msg, VALID_STATES};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateWorkUnitStatusArgs {
    work_unit_id: String,
    status: String,
    #[serde(default)]
    blocked_reason: Option<String>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    skip_temporal_validation: Option<bool>,
}

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: UpdateWorkUnitStatusArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "update-work-unit-status",
            reason: format!("failed to parse args: {e}"),
        })?;
    change_status(
        project_root,
        StatusChange {
            id: args.work_unit_id,
            new_status: args.status,
            blocked_reason: args.blocked_reason,
            reason: args.reason,
            skip_temporal: args.skip_temporal_validation.unwrap_or(false),
            discard_of: None,
        },
    )
}

/// WT-018: move `id` back to `status`, its status before session
/// `session_id`, as part of discarding the session's worktree. The only
/// path back to backlog: the backlog gate and the ACDD transition order do
/// not apply (the card held `status` before the session), every other
/// gate does, and the stateHistory entry records the move as a discard of
/// the session. `blocked_reason` restores a blocked status's reason.
pub(crate) fn discard_transition(
    project_root: &Path,
    id: &str,
    status: &str,
    blocked_reason: Option<&str>,
    session_id: &str,
) -> Result<String, FspecCoreError> {
    change_status(
        project_root,
        StatusChange {
            id: id.to_string(),
            new_status: status.to_string(),
            blocked_reason: blocked_reason.map(str::to_string),
            reason: None,
            skip_temporal: false,
            discard_of: Some(session_id),
        },
    )
}

/// One requested status change.
struct StatusChange<'a> {
    id: String,
    new_status: String,
    blocked_reason: Option<String>,
    reason: Option<String>,
    skip_temporal: bool,
    /// WT-018: the session whose worktree is being discarded.
    discard_of: Option<&'a str>,
}

fn change_status(project_root: &Path, change: StatusChange<'_>) -> Result<String, FspecCoreError> {
    let StatusChange {
        id,
        new_status,
        blocked_reason,
        reason,
        skip_temporal,
        discard_of,
    } = change;

    // Validate state is allowed.
    if !VALID_STATES.contains(&new_status.as_str()) {
        return Err(msg(format!(
            "Invalid status value: {}. Allowed values: {}",
            new_status,
            VALID_STATES.join(", ")
        )));
    }

    // BUG-477: hold the spec lock from the first read to the last write.
    let _spec_lock = crate::io::spec_lock::lock_spec_dir(project_root, "update-work-unit-status")?;
    let mut data = ensure_work_units_file(project_root)?;

    // Check if work unit exists.
    if !data.work_units.contains_key(&id) {
        return Err(msg(format!("Work unit '{id}' does not exist")));
    }

    let (current_status, work_type) = {
        let wu = &data.work_units[&id];
        (
            wu.status.as_str().to_string(),
            // BUG-385: a missing type is malformed, never a story.
            wu.r#type
                .clone()
                .ok_or_else(|| queries::malformed(&id, "type", "missing"))?,
        )
    };

    let is_task = work_type == "task";

    check_transition_gates(
        &data,
        &id,
        &current_status,
        &new_status,
        is_task,
        blocked_reason.as_deref(),
        discard_of.is_some(),
    )?;

    // GATE: prefill in linked feature files (blocks ALL forward transitions except to blocked).
    if new_status != "blocked" && current_status != new_status {
        check_prefill(project_root, &id)?;
    }

    // Warnings accumulated for the success output.
    let mut warnings: Vec<String> = Vec::new();

    // GATE: prerequisites for testing state (only when leaving specifying).
    let review_reminder = if new_status == "testing" && current_status == "specifying" {
        check_testing_entry(
            project_root,
            &data,
            &id,
            &work_type,
            skip_temporal,
            &mut warnings,
        )?
    } else {
        None
    };

    // GATE: temporal validation for implementing state (tests created after entering testing).
    if new_status == "implementing" && current_status == "testing" && !skip_temporal && !is_task {
        check_temporal_ordering(project_root, &data, &id, &new_status)?;
    }

    // GATE (AUDIT-005): the audit core over the work unit's features. The
    // coverage area on the way to implementing or validating (tasks are
    // exempt), with an implementation mapping required before validating.
    // It runs before the post-status hooks (GATE 8), so a blocked move runs
    // no hook.
    if (new_status == "implementing" || new_status == "validating") && !is_task {
        check_audit(
            project_root,
            &id,
            &new_status,
            &[Area::Coverage],
            new_status == "validating",
        )?;
    }

    // GATE: done finalization — parent/child constraints first, then the
    // coverage and spec areas of the audit (AUDIT-005).
    if new_status == "done" {
        let incomplete = incomplete_children(&data, &id)?;
        if !incomplete.is_empty() {
            return Err(msg(format!(
                "Cannot mark parent as done while children are incomplete: {}. Complete all children first.",
                incomplete.join(", ")
            )));
        }
        check_audit(
            project_root,
            &id,
            &new_status,
            &[Area::Coverage, Area::Spec],
            false,
        )?;
    }

    // GATE 8 (HOOK-019, HOOK-020): post-<current> then pre-<new> status
    // hooks, global hooks before the work unit's virtual hooks; a failing
    // blocking hook blocks the move.
    let hook_output = crate::command_hooks::run_status_hooks(
        project_root,
        &data,
        &id,
        &current_status,
        &new_status,
    )?;

    // GATE 9: auto-checkpoint before transition (dirty & not →backlog & not from backlog).
    if current_status != "backlog" && new_status != "backlog" {
        let checkpoint_name = format!("{id}-auto-{current_status}");
        maybe_auto_checkpoint(project_root, &id, &checkpoint_name)?;
    }

    // BUG-385: validated before the move is saved.
    let virtual_hooks = array_len(&data, &id, "virtualHooks")?;

    // Apply the status change.
    let now = iso8601_now();
    apply_status_change(
        &mut data,
        &id,
        &new_status,
        blocked_reason.as_deref(),
        reason.as_deref(),
        discard_of,
        &now,
    )?;

    // Move the id between state arrays.
    move_state(&mut data, &id, &current_status, &new_status);

    if let Some(meta) = data.meta.as_mut() {
        meta.last_updated = now;
    }

    // GATE 10: done finalization — inline compaction.
    if new_status == "done" {
        compact_done(&mut data, &id);
    }

    crate::io::board::write_work_units(project_root, &data)?;

    // Cleanup auto-checkpoints on →done, preserving manual ones.
    if new_status == "done" {
        cleanup_done_checkpoints(project_root, &id);
    }
    let rendered = render_output(
        project_root,
        &data,
        &Transition {
            id: &id,
            current_status: &current_status,
            new_status: &new_status,
            work_type: &work_type,
            virtual_hooks,
        },
        review_reminder,
        &warnings,
    );
    Ok(if hook_output.is_empty() {
        rendered
    } else {
        format!("{}\n{hook_output}", rendered.trim_end_matches('\n'))
    })
}
