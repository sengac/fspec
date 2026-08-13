//! `list-epics` help configuration — Rust port of
//! `src/commands/list-epics-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js list-epics --help`.

use super::super::{CommandExample, CommandHelpConfig};

const EXAMPLE_OUTPUT: &str =
    "user-management - User Management Features\nauth - Authentication System\n\nFound 2 epics";

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec list-epics",
    description: Some("List all epics"),
    output: Some(EXAMPLE_OUTPUT),
}];

const RELATED: &[&str] = &["create-epic", "show-epic", "delete-epic"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "list-epics",
    description: "List all epics in the project",
    usage: Some("fspec list-epics"),
    arguments: &[],
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
