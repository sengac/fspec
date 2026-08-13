//! `delete-step` help configuration — Rust port of
//! `src/commands/delete-step-help.ts`.
//!
//! The output of `format_command_help(&CONFIG)` is byte-for-byte identical
//! to `node dist/index.js delete-step --help` when piped to non-TTY.
//!
//! Captured fixture: `rust/fspec/tests/fixtures/help/delete-step.txt`
//!
//! Note: the TS help config documents three positional arguments
//! (`file`, `scenario`, `index`) and a `--force` flag. The TS Commander.js
//! registration only wires `<feature> <scenario> <step>`, but the help
//! page is rendered verbatim from this config, so the Rust port mirrors the
//! config (not the runtime registration) for byte-for-byte fixture parity.

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
        name: "index",
        description: "Step index (0-based)",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "--force",
    description: "Skip confirmation",
    default_value: None,
}];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec delete-step spec/features/login.feature \"Login with valid credentials\" 2",
    description: Some("Delete step"),
    output: Some("✓ Deleted step from scenario"),
}];

const RELATED: &[&str] = &["add-step", "update-step"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "delete-step",
    description: "Delete a step from a scenario",
    usage: Some("fspec delete-step <file> <scenario> <index> [options]"),
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
