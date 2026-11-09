//! State-transition rules and the request gates of `update-work-unit-status`.

use super::queries::{collect_active_blockers, Data};
use crate::error::FspecCoreError;
use crate::types::work_unit::WorkUnitStatus;

pub(super) const VALID_STATES: &[&str] = &[
    "backlog",
    "specifying",
    "testing",
    "implementing",
    "validating",
    "done",
    "blocked",
];

pub(super) fn allowed_transitions(from: &str) -> &'static [&'static str] {
    // Mirrors STATE_TRANSITIONS in src/commands/update-work-unit-status.ts.
    match from {
        "backlog" => &["specifying", "blocked"],
        "specifying" => &["testing", "blocked"],
        "testing" => &["implementing", "specifying", "blocked"],
        "implementing" => &["validating", "testing", "specifying", "blocked"],
        "validating" => &["done", "implementing", "testing", "specifying", "blocked"],
        "done" => &[
            "specifying",
            "testing",
            "implementing",
            "validating",
            "blocked",
        ],
        "blocked" => &[
            "backlog",
            "specifying",
            "testing",
            "implementing",
            "validating",
        ],
        _ => &[],
    }
}

pub(super) fn status_from_str(s: &str) -> Option<WorkUnitStatus> {
    match s {
        "backlog" => Some(WorkUnitStatus::Backlog),
        "specifying" => Some(WorkUnitStatus::Specifying),
        "testing" => Some(WorkUnitStatus::Testing),
        "implementing" => Some(WorkUnitStatus::Implementing),
        "validating" => Some(WorkUnitStatus::Validating),
        "done" => Some(WorkUnitStatus::Done),
        "blocked" => Some(WorkUnitStatus::Blocked),
        _ => None,
    }
}

/// Plain message error that is NOT wrapped by the args-validation prefix.
pub(super) fn msg(reason: impl Into<String>) -> FspecCoreError {
    FspecCoreError::Message(reason.into())
}

/// The gates every requested transition passes before any file is read:
/// tasks have no testing phase, nothing returns to backlog, blocked needs a
/// reason, the transition must be allowed for the work type, and active
/// states need every blocking dependency done. A WT-018 `discard` restores
/// the card's status from before a discarded session, so neither the
/// backlog gate nor the transition order applies to it.
pub(super) fn check_transition_gates(
    data: &Data,
    id: &str,
    current_status: &str,
    new_status: &str,
    is_task: bool,
    blocked_reason: Option<&str>,
    discard: bool,
) -> Result<(), FspecCoreError> {
    // GATE: tasks have no testing phase.
    if is_task && new_status == "testing" {
        return Err(msg(
            "Tasks do not have a testing phase. task workflow: backlog → specifying → implementing → validating → done.\n\
Tasks are for operational work without testable acceptance criteria.\n\
Use stories for user-facing features that require tests.",
        ));
    }

    // GATE: prevent moving back to backlog.
    if new_status == "backlog" && current_status != "backlog" && !discard {
        return Err(msg(
            "Cannot move work back to backlog. Use 'blocked' state if work cannot progress.",
        ));
    }

    // GATE: blocked state requires a reason.
    if new_status == "blocked" && blocked_reason.is_none_or(str::is_empty) {
        return Err(msg(
            "Blocked reason is required when moving to blocked state. Use --blocked-reason='description of blocker'",
        ));
    }

    // GATE: validate state transitions (ACDD enforcement with type-specific rules).
    if current_status != new_status && !discard {
        // Special case for tasks: allow specifying → implementing (skip testing).
        let is_task_skipping_test =
            is_task && current_status == "specifying" && new_status == "implementing";
        if !is_task_skipping_test && !allowed_transitions(current_status).contains(&new_status) {
            let mut parts = vec![format!(
                "Invalid state transition from '{current_status}' to '{new_status}'."
            )];
            if current_status == "backlog" && new_status == "testing" {
                parts.push("Must move to 'specifying' state first.".to_string());
                parts.push("ACDD requires specification before testing.".to_string());
            } else if current_status == "specifying" && new_status == "implementing" && !is_task {
                parts.push("Must move to 'testing' state first.".to_string());
                parts.push("ACDD requires tests before implementation.".to_string());
                parts.push(
                    "Note: Only tasks can skip testing. Use --type=task for operational work."
                        .to_string(),
                );
            }
            return Err(msg(parts.join(" ")));
        }
    }

    // GATE: prevent starting work blocked by incomplete dependencies.
    let active_states = ["specifying", "testing", "implementing", "validating"];
    if new_status != "blocked" && active_states.contains(&new_status) {
        let blockers = collect_active_blockers(data, id)?;
        if !blockers.is_empty() {
            return Err(msg(format!(
                "Cannot start work on {id}: work unit is blocked by incomplete dependencies.\n\n\
Active blockers:\n  - {}\n\n\
Complete blocking work units or remove dependencies before starting work.",
                blockers.join("\n  - ")
            )));
        }
    }

    Ok(())
}
