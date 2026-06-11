//! `add-assumption` help configuration — Rust port of
//! `src/commands/add-assumption-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js add-assumption --help`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "work-unit-id",
        description: "Work unit ID",
        required: true,
    },
    CommandArgument {
        name: "assumption",
        description: "Assumption text",
        required: true,
    },
];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec add-assumption AUTH-001 \"Users have valid email addresses\"",
    description: Some("Add assumption"),
    output: Some("✓ Assumption added successfully"),
}];

const RELATED: &[&str] = &["add-rule", "add-question", "show-work-unit"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-assumption",
    description: "Add an assumption to a work unit during specification",
    usage: Some("fspec add-assumption <work-unit-id> <assumption>"),
    arguments: ARGS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use to document assumptions made during specification that may need validation later.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: &[],
};
