//! `list-schedules` help configuration — Rust port of
//! `src/commands/list-schedules-help.ts`.
//!
//! The output of `format_command_help(&CONFIG)` is byte-for-byte identical
//! to `node dist/index.js list-schedules --help` when piped to non-TTY.
//!
//! Captured fixture: `rust/fspec/tests/fixtures/help/list-schedules.txt`

use super::super::{CommandExample, CommandHelpConfig, CommandOption, CommonError};

const EXAMPLE_1_OUTPUT: &str = "Name            Cron          Timezone          Type    Status  Last Run\n----------------------------------------------------------------------------------------------------\nnightly-review  0 2 * * *     UTC               agent   active  2025-01-15 02:00\ndaily-tests     30 6 * * 1-5  America/New_York  shell   active  2025-01-15 06:30\n\nTotal: 2 schedule(s)";

const EXAMPLE_2_OUTPUT: &str =
    "[{\"name\":\"nightly-review\",\"cron\":\"0 2 * * *\",\"timezone\":\"UTC\",...}]";

const EXAMPLE_3_OUTPUT: &str =
    "No schedules configured.\nUse `fspec add-schedule` to create a schedule.";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec list-schedules",
        description: Some("List all schedules in table format"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec list-schedules --json",
        description: Some("List schedules as JSON for programmatic use"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
    CommandExample {
        command: "fspec list-schedules",
        description: Some("When no schedules are configured"),
        output: Some(EXAMPLE_3_OUTPUT),
    },
];

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "--json",
    description: "Output schedule data as JSON instead of a formatted table.",
    default_value: None,
}];

const COMMON_ERRORS: &[CommonError] = &[CommonError {
    error: "No schedules configured",
    fix: "This is not an error. Create a schedule with 'fspec add-schedule'.",
}];

// Note: TS source lists relatedCommands with embedded " - description" suffix,
// which formatCommandHelp emits verbatim. We preserve that quirk for byte-parity.
const RELATED: &[&str] = &[
    "add-schedule - Create a new schedule",
    "remove-schedule - Remove a schedule",
    "pause-schedule - Pause an active schedule",
    "resume-schedule - Resume a paused schedule",
];

const NOTES: &[&str] = &[
    "Returns an empty list (not an error) when no schedules exist",
    "The --json flag is useful for scripting and CI/CD integration",
    "Status column shows \"active\" (green) or \"paused\" (yellow)",
    "Last Run shows the timestamp of the most recent execution",
    "Schedules are stored in spec/schedules.json",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "list-schedules",
    description: "List all configured scheduled jobs from spec/schedules.json. Shows schedule name, cron expression, timezone, job type, status, and last run information.",
    usage: Some("fspec list-schedules [options]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use to review all configured schedules, check their status (active/paused), verify cron expressions, or find schedule names for other schedule commands.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: Some(
        "fspec list-schedules → review status → fspec pause-schedule / fspec resume-schedule / fspec remove-schedule as needed",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
