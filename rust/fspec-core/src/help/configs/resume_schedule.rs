//! `resume-schedule` help configuration — Rust port of the TS
//! `resume-schedule` Commander.js `--help` reference.
//!
//! Byte-for-byte parity with `node dist/index.js resume-schedule --help`
//! (captured at `rust/fspec/tests/fixtures/help/resume-schedule.txt`).

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommonError};

const PREREQUISITES: &[&str] = &[
    "Schedule must exist in spec/schedules.json",
    "Schedule must currently have status \"paused\"",
];

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "name",
    description: "The name of the schedule to resume (must be currently paused)",
    required: true,
}];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec resume-schedule nightly-review",
        description: Some("Resume the nightly-review schedule after maintenance"),
        output: Some("✓ Schedule 'nightly-review' resumed successfully"),
    },
    CommandExample {
        command: "fspec resume-schedule daily-tests",
        description: Some("Re-enable daily test runs"),
        output: Some("✓ Schedule 'daily-tests' resumed successfully"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Schedule 'nightly-review' does not exist",
        fix: "Check the schedule name with 'fspec list-schedules'. Names are case-sensitive slugs.",
    },
    CommonError {
        error: "Schedule 'nightly-review' is already active",
        fix: "The schedule is already active and will trigger normally. No action needed.",
    },
];

const RELATED: &[&str] = &[
    "pause-schedule - Pause an active schedule",
    "list-schedules - List all schedules and their status",
    "add-schedule - Create a new schedule",
    "remove-schedule - Permanently remove a schedule",
];

const NOTES: &[&str] = &[
    "Resuming restores the schedule to \"active\" status",
    "The schedule will trigger on its next matching cron time after resuming",
    "Missed runs during the paused period are not retroactively executed",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "resume-schedule",
    description: "Resume a paused scheduled job by setting its status back to \"active\". The schedule will begin triggering again according to its cron expression.",
    usage: Some("fspec resume-schedule <name>"),
    arguments: ARGUMENTS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some("Use after a maintenance window or investigation is complete to re-enable a previously paused schedule."),
    when_not_to_use: Some("Do not use on schedules that are already active. Check status first with list-schedules."),
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    typical_workflow: Some("fspec list-schedules → confirm schedule is paused → fspec resume-schedule <name> → fspec list-schedules (verify active)"),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
