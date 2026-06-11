//! `add-rule` help configuration — Rust port of
//! `src/commands/add-rule-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js add-rule --help`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "Work unit ID",
        required: true,
    },
    CommandArgument {
        name: "rule",
        description: "Business rule description",
        required: true,
    },
];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec add-rule AUTH-001 \"Email must be valid format\"",
    description: Some("Add business rule"),
    output: Some("✓ Rule added successfully"),
}];

const RELATED: &[&str] = &[
    "add-question",
    "add-example",
    "generate-scenarios",
    "remove-rule",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-rule",
    description: "Add a business rule to a work unit during Example Mapping",
    usage: Some("fspec add-rule <workUnitId> <rule>"),
    arguments: ARGS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use during specifying phase when capturing business rules discovered through Example Mapping conversations.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: &[],
};
