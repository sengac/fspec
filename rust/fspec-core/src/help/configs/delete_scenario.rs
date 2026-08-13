//! `delete-scenario` help configuration — Rust port of
//! `src/commands/delete-scenario-help.ts`.
//!
//! The output of `format_command_help(&CONFIG)` is byte-for-byte identical
//! to `node dist/index.js delete-scenario --help` when piped to non-TTY.
//!
//! Captured fixture: `rust/fspec/tests/fixtures/help/delete-scenario.txt`

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGUMENTS: &[CommandArgument] = &[
    CommandArgument {
        name: "file",
        description: "Feature file path",
        required: true,
    },
    CommandArgument {
        name: "scenario",
        description: "Scenario name to delete",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "--force",
    description: "Skip confirmation",
    default_value: None,
}];

const EXAMPLE_OUTPUT: &str =
    "✓ Deleted scenario \"Deprecated scenario\" from login.feature\n  Updated coverage file";

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec delete-scenario spec/features/login.feature \"Deprecated scenario\"",
    description: Some("Delete scenario and update coverage file"),
    output: Some(EXAMPLE_OUTPUT),
}];

const RELATED: &[&str] = &["add-scenario", "delete-scenarios"];

const NOTES: &[&str] = &[
    "Scenario name must match exactly (case-sensitive)",
    "Automatically removes scenario from .feature.coverage file if it exists",
    "Coverage statistics (totalScenarios, coveredScenarios, coveragePercent) are recalculated",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "delete-scenario",
    description: "Delete a scenario from a feature file",
    usage: Some("fspec delete-scenario <file> <scenario> [options]"),
    arguments: ARGUMENTS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: None,
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: NOTES,
};
