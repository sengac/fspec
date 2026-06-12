//! `record-iteration` help configuration — Rust port of
//! `src/commands/record-iteration-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js record-iteration --help`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGS: &[CommandArgument] = &[CommandArgument {
    name: "name",
    description: "Iteration name (e.g., \"Sprint 1\", \"Week 42\")",
    required: true,
}];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--start <date>",
        description: "Start date (ISO format)",
        default_value: None,
    },
    CommandOption {
        flag: "--end <date>",
        description: "End date (ISO format)",
        default_value: None,
    },
];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec record-iteration \"Sprint 1\" --start 2025-10-01 --end 2025-10-15",
    description: Some("Record iteration"),
    output: Some("✓ Recorded iteration \"Sprint 1\""),
}];

const RELATED: &[&str] = &["query-metrics", "generate-summary-report"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "record-iteration",
    description: "Record an iteration or sprint with metadata",
    usage: Some("fspec record-iteration <name> [options]"),
    arguments: ARGS,
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
