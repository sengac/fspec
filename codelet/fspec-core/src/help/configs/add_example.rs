//! `add-example` help configuration — Rust port of
//! `src/commands/add-example-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js add-example --help`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig};

const ARGUMENTS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "Work unit ID",
        required: true,
    },
    CommandArgument {
        name: "example",
        description: "Concrete example description",
        required: true,
    },
];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec add-example AUTH-001 \"Login with user@example.com and correct password\"",
    description: Some("Add example"),
    output: Some("✓ Example added successfully"),
}];

const RELATED: &[&str] = &[
    "add-rule",
    "add-question",
    "generate-scenarios",
    "remove-example",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-example",
    description: "Add a concrete example to a work unit during Example Mapping",
    usage: Some("fspec add-example <workUnitId> <example>"),
    arguments: ARGUMENTS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use during specifying phase to capture concrete examples that illustrate rules and will become test scenarios.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: &[],
};
