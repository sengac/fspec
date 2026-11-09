//! System-reminder generation for `update-work-unit-status`.
//!
//! Faithful port of the reminder behaviour in
//! `src/utils/system-reminder.ts` + the inline reminders constructed in
//! `src/commands/update-work-unit-status.ts`. All reminders are gated by
//! `FSPEC_DISABLE_REMINDERS != "1"` (mirroring `isRemindersEnabled`).
//!
//! Templates are stored as raw-string constants using the `__ID__` token in
//! place of the TS `${workUnitId}` interpolation; [`fill`] substitutes the id.

mod configure_tools;
mod enhanced;
mod specifying;
mod status;

pub(crate) use configure_tools::{check_quality_commands, check_test_command};
pub(super) use enhanced::subjective_review_reminder;
pub(super) use status::{
    done_review_reminder, event_storm_reminder, status_change_reminder,
    virtual_hooks_cleanup_reminder, virtual_hooks_reminder,
};

type Data = crate::types::work_unit::WorkUnitsData;

/// Mirrors `isRemindersEnabled()`.
pub(super) fn reminders_enabled() -> bool {
    std::env::var("FSPEC_DISABLE_REMINDERS")
        .map(|v| v != "1")
        .unwrap_or(true)
}

/// Mirrors `wrapInSystemReminder`.
pub(super) fn wrap(content: &str) -> String {
    format!("<system-reminder>\n{content}\n</system-reminder>")
}

/// Substitute the `__ID__` token with the work-unit id.
fn fill(template: &str, id: &str) -> String {
    template.replace("__ID__", id)
}

/// CLI-015: the AST-search block of the specifying reminder, rendered per
/// mode. Harness (capture) mode names the native AstGrep tool; CLI mode
/// names the `fspec astgrep` subcommand.
fn ast_research_block(in_capture: bool) -> &'static str {
    if in_capture {
        "For AST code search and refactoring, use the AstGrep and AstGrepRefactor tools:\n  AstGrep(language=\"typescript\", pattern=\"function $NAME($$$ARGS) { $$$BODY }\", path=\"src/\")"
    } else {
        "For AST code search and refactoring, use the `fspec astgrep` command:\n  fspec astgrep --pattern 'function $NAME($$$ARGS) { $$$BODY }' --lang typescript [--path src/]"
    }
}

/// Mirrors `consolidateReminders`: strip individual wrapper tags, trim, drop
/// empties, join with a blank line, and re-wrap once.
pub(super) fn consolidate(reminders: &[String]) -> Option<String> {
    if reminders.is_empty() {
        return None;
    }
    let mut contents: Vec<String> = Vec::new();
    for r in reminders {
        let stripped = r
            .replace("<system-reminder>\n", "")
            .replace("<system-reminder>", "")
            .replace("</system-reminder>\n", "")
            .replace("</system-reminder>", "");
        let trimmed = stripped.trim().to_string();
        if !trimmed.is_empty() {
            contents.push(trimmed);
        }
    }
    if contents.is_empty() {
        return None;
    }
    Some(wrap(&contents.join("\n\n")))
}

#[cfg(test)]
mod tests;
