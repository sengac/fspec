//! `remove-schedule` help configuration — Rust port of
//! `src/commands/remove-schedule-help.ts`.
//!
//! The output of `format_command_help(&CONFIG)` is byte-for-byte identical to
//! `node dist/index.js remove-schedule --help` when piped to non-TTY.
//!
//! Captured fixture: `codelet/fspec/tests/fixtures/help/remove-schedule.txt`

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommonError};

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec remove-schedule nightly-review",
        description: Some("Remove the nightly-review schedule"),
        output: Some("✓ Schedule 'nightly-review' removed successfully"),
    },
    CommandExample {
        command: "fspec remove-schedule daily-tests",
        description: Some("Remove the daily-tests schedule"),
        output: Some("✓ Schedule 'daily-tests' removed successfully"),
    },
];

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "name",
    description: "The name of the schedule to remove (must match an existing schedule)",
    required: true,
}];

const PREREQUISITES: &[&str] = &[
    "Schedule must exist in spec/schedules.json",
    "spec/schedules.json must exist (created by add-schedule)",
];

const COMMON_ERRORS: &[CommonError] = &[CommonError {
    error: "Schedule 'nightly-review' does not exist",
    fix: "Check the schedule name with 'fspec list-schedules'. Names are case-sensitive slugs.",
}];

const RELATED: &[&str] = &[
    "add-schedule - Create a new schedule",
    "list-schedules - List all configured schedules",
    "pause-schedule - Temporarily disable a schedule (non-destructive)",
    "resume-schedule - Re-enable a paused schedule",
];

const NOTES: &[&str] = &[
    "Removal is permanent — there is no undo",
    "Consider using pause-schedule if you may need the schedule again",
    "The schedule entry is deleted from spec/schedules.json atomically",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "remove-schedule",
    description: "Remove a scheduled job from spec/schedules.json. Permanently deletes the schedule entry.",
    usage: Some("fspec remove-schedule <name>"),
    arguments: ARGUMENTS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use when a scheduled job is no longer needed and should be permanently removed. For temporary disabling, use pause-schedule instead.",
    ),
    when_not_to_use: Some(
        "Do not use if you only want to temporarily stop a schedule. Use pause-schedule to suspend it and resume-schedule to reactivate later.",
    ),
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    typical_workflow: Some(
        "fspec list-schedules → identify obsolete schedule → fspec remove-schedule <name> → fspec list-schedules (verify)",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
