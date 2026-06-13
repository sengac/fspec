//! `get-scenarios` help configuration — Rust port of
//! `src/commands/get-scenarios-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js get-scenarios --help`.
//! Captured fixture: `codelet/fspec/tests/fixtures/help/get-scenarios.txt`.
//!
//! NOTE: the `--file <file>` option is part of the TS help config (and thus
//! the byte-exact fixture) even though the clap surface only implements
//! `--tag` / `--format`. The help text is documentation parity, not a binding
//! contract on the implemented flags.

use super::super::{CommandExample, CommandHelpConfig, CommandOption};

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--file <file>",
        description: "Specific feature file",
        default_value: None,
    },
    CommandOption {
        flag: "--tag <tag>",
        description: "Filter by tag (can use multiple)",
        default_value: None,
    },
    CommandOption {
        flag: "--format <format>",
        description: "Output format: text, json, or list",
        default_value: None,
    },
];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec get-scenarios --file spec/features/login.feature",
    description: Some("Get scenarios from file"),
    output: Some(
        "Login with valid credentials\nLogin with invalid password\nLogin with locked account",
    ),
}];

const RELATED: &[&str] = &["show-acceptance-criteria", "list-features"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "get-scenarios",
    description: "Extract scenarios from feature files with optional filtering",
    usage: Some("fspec get-scenarios [options]"),
    arguments: &[],
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
