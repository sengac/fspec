//! `remove-rule` help configuration — Rust port of
//! `src/commands/remove-rule-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js remove-rule --help`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "Work unit ID",
        required: true,
    },
    CommandArgument {
        name: "index",
        description: "Rule index (from show-work-unit)",
        required: true,
    },
];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec remove-rule AUTH-001 1",
    description: Some("Remove rule at index 1"),
    output: Some("✓ Removed rule from AUTH-001"),
}];

const RELATED: &[&str] = &["add-rule", "show-work-unit"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "remove-rule",
    description: "Remove a business rule from Example Mapping by index",
    usage: Some("fspec remove-rule <workUnitId> <index>"),
    arguments: ARGS,
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
