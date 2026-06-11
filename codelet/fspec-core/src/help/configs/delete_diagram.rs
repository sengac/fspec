//! `delete-diagram` help configuration — Rust port of
//! `src/commands/delete-diagram-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js delete-diagram --help`.
//! Captured fixture: `codelet/fspec/tests/fixtures/help/delete-diagram.txt`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "section",
        description: "Section name in foundation.json",
        required: true,
    },
    CommandArgument {
        name: "title",
        description: "Diagram title to delete",
        required: true,
    },
];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec delete-diagram \"Architecture Diagrams\" \"Command Flow\"",
    description: Some("Delete specific diagram"),
    output: Some("✓ Deleted diagram from foundation.json"),
}];

const RELATED: &[&str] = &["add-diagram", "show-foundation", "update-foundation"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "delete-diagram",
    description: "Delete a diagram from foundation.json",
    usage: Some("fspec delete-diagram <section> <title>"),
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
