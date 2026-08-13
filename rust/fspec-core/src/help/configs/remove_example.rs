//! `remove-example` help configuration — Rust port of
//! `src/commands/remove-example-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js remove-example --help`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig};

const ARGUMENTS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "Work unit ID",
        required: true,
    },
    CommandArgument {
        name: "index",
        description: "Example index (from show-work-unit)",
        required: true,
    },
];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec remove-example AUTH-001 2",
    description: Some("Remove example at index 2"),
    output: Some("✓ Removed example from AUTH-001"),
}];

const RELATED: &[&str] = &["add-example", "show-work-unit"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "remove-example",
    description: "Remove an example from Example Mapping by index",
    usage: Some("fspec remove-example <workUnitId> <index>"),
    arguments: ARGUMENTS,
    options: &[],
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
