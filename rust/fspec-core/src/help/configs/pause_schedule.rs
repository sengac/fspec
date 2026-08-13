//! `pause-schedule` help configuration — Rust port of the TS
//! `pause-schedule` Commander.js `--help` reference.
//!
//! Byte-for-byte parity with `node dist/index.js pause-schedule --help`
//! (captured at `rust/fspec/tests/fixtures/help/pause-schedule.txt`).

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommonError};

const PREREQUISITES: &[&str] = &[
    "Schedule must exist in spec/schedules.json",
    "Schedule must currently have status \"active\"",
];

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "name",
    description: "The name of the schedule to pause (must be currently active)",
    required: true,
}];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec pause-schedule nightly-review",
        description: Some("Pause the nightly-review schedule"),
        output: Some("✓ Schedule 'nightly-review' paused successfully"),
    },
    CommandExample {
        command: "fspec pause-schedule daily-tests",
        description: Some("Pause during a maintenance window"),
        output: Some("✓ Schedule 'daily-tests' paused successfully"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Schedule 'nightly-review' does not exist",
        fix: "Check the schedule name with 'fspec list-schedules'. Names are case-sensitive slugs.",
    },
    CommonError {
        error: "Schedule 'nightly-review' is already paused",
        fix: "The schedule is already paused. Use resume-schedule to reactivate it.",
    },
];

const RELATED: &[&str] = &[
    "resume-schedule - Resume a paused schedule",
    "list-schedules - List all schedules and their status",
    "remove-schedule - Permanently remove a schedule",
    "add-schedule - Create a new schedule",
];

const NOTES: &[&str] = &[
    "Pausing is non-destructive — the schedule configuration is preserved",
    "A paused schedule will not trigger until explicitly resumed",
    "Use list-schedules to verify the schedule status after pausing",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "pause-schedule",
    description: "Pause a scheduled job by setting its status to \"paused\". The schedule remains in spec/schedules.json but will not trigger until resumed.",
    usage: Some("fspec pause-schedule <name>"),
    arguments: ARGUMENTS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some("Use when you want to temporarily stop a schedule from triggering without removing it. Useful during maintenance windows, deployments, or when investigating issues."),
    when_not_to_use: Some("Do not use if you want to permanently remove the schedule. Use remove-schedule instead."),
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    typical_workflow: Some("fspec list-schedules → fspec pause-schedule <name> (before maintenance) → perform maintenance → fspec resume-schedule <name> (after maintenance)"),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
