//! `delete-step` help configuration — Rust port of
//! `src/commands/delete-step-help.ts`.
//!
//! The output of `format_command_help(&CONFIG)` is byte-for-byte identical
//! to `node dist/index.js delete-step --help` when piped to non-TTY.
//!
//! Captured fixture: `rust/fspec/tests/fixtures/help/delete-step.txt`
//!
//! BUG-505: the TS help config documented a 0-based `index` argument and a
//! `--force` flag, but the command matches the step by its text and has no
//! confirmation; the help now describes the real `<step>` argument.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGUMENTS: &[CommandArgument] = &[
    CommandArgument {
        name: "file",
        description: "Feature file path",
        required: true,
    },
    CommandArgument {
        name: "scenario",
        description: "Scenario name",
        required: true,
    },
    CommandArgument {
        name: "step",
        description: "Step text, with or without its keyword (e.g. \"Given I am logged in\" or \"I am logged in\"); text matching several steps is refused, so add the keyword to pick one",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec delete-step spec/features/login.feature \"Login with valid credentials\" \"And I see the dashboard\"",
    description: Some("Delete step (its data table or doc string goes with it)"),
    output: Some("✓ Deleted step from scenario"),
}];

const RELATED: &[&str] = &["add-step", "update-step"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "delete-step",
    description: "Delete a step from a scenario",
    usage: Some("fspec delete-step <file> <scenario> <step>"),
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
    notes: &[],
};
