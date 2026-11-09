//! Success output of `update-work-unit-status`: system reminders in TS order
//! consolidated into one block, warnings and the confirmation line.

use std::path::Path;

use super::queries::Data;
use super::reminders;

/// The work unit and the transition that was applied.
pub(super) struct Transition<'a> {
    pub(super) id: &'a str,
    pub(super) current_status: &'a str,
    pub(super) new_status: &'a str,
    pub(super) work_type: &'a str,
    /// BUG-385: read and validated before the move is saved.
    pub(super) virtual_hooks: usize,
}

/// Build the stdout of a successful transition.
pub(super) fn render_output(
    project_root: &Path,
    data: &Data,
    transition: &Transition<'_>,
    review_reminder: Option<String>,
    warnings: &[String],
) -> String {
    let Transition {
        id,
        current_status,
        new_status,
        work_type,
        virtual_hooks,
    } = *transition;
    // Collect system reminders in TS order, then consolidate into one block.
    let mut reminder_parts: Vec<String> = Vec::new();
    if let Some(r) = reminders::status_change_reminder(id, new_status, data, project_root) {
        reminder_parts.push(r);
    }
    if new_status == "specifying" {
        reminder_parts.push(reminders::event_storm_reminder(id));
    }
    if current_status == "specifying" && new_status == "testing" {
        if let Some(r) = reminders::virtual_hooks_reminder(id) {
            reminder_parts.push(r);
        }
    }
    if let Some(r) = review_reminder {
        reminder_parts.push(r);
    }
    if new_status == "done" {
        if let Some(r) = reminders::virtual_hooks_cleanup_reminder(id, virtual_hooks) {
            reminder_parts.push(r);
        }
        if let Some(r) = reminders::done_review_reminder(id, work_type) {
            reminder_parts.push(r);
        }
    }
    let consolidated = reminders::consolidate(&reminder_parts);

    // Build stdout to mirror the TS CLI exactly:
    //   1. (validating only) configure-tools test + quality check messages
    //   2. `✓ ...` confirmation line
    //   3. one `⚠ <warning>` line each
    //   4. blank line + consolidated `<system-reminder>` block
    let mut out = String::new();
    if new_status == "validating" {
        out.push_str(&reminders::check_test_command(project_root));
        out.push('\n');
        out.push_str(&reminders::check_quality_commands(project_root));
        out.push('\n');
    }
    out.push_str(&format!(
        "✓ Work unit {id} status updated to {new_status}\n"
    ));
    for w in warnings {
        out.push_str(&format!("⚠ {w}\n"));
    }
    if let Some(reminder) = consolidated {
        out.push('\n');
        out.push_str(&reminder);
        out.push('\n');
    }
    out
}
