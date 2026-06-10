//! `create-prefix` help configuration — Rust port of
//! `src/commands/create-prefix-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js create-prefix --help`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig};

const EXAMPLE_OUTPUT: &str = "✓ Created prefix AUTH\n  Description: Authentication features";

const ARGUMENTS: &[CommandArgument] = &[
    CommandArgument {
        name: "prefix",
        description: "Prefix code (e.g., AUTH, UI, API) - uppercase, short",
        required: true,
    },
    CommandArgument {
        name: "description",
        description: "Description of what this prefix represents",
        required: true,
    },
];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec create-prefix AUTH \"Authentication features\"",
    description: Some("Register new prefix"),
    output: Some(EXAMPLE_OUTPUT),
}];

const RELATED: &[&str] = &[
    "list-prefixes",
    "update-prefix",
    "create-story",
    "create-bug",
    "create-task",
];

const NOTES: &[&str] = &[
    "Prefix must be uppercase",
    "Required before creating work units with that prefix",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "create-prefix",
    description: "Register a new work unit prefix for organizing work by component or area",
    usage: Some("fspec create-prefix <prefix> <description>"),
    arguments: ARGUMENTS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use when starting work on a new component or area that needs its own work unit namespace.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: NOTES,
};
