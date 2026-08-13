//! `list-prefixes` help configuration — Rust port of
//! `src/commands/list-prefixes-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js list-prefixes --help`.

use super::super::{CommandExample, CommandHelpConfig};

const EXAMPLE_OUTPUT: &str = "AUTH - Authentication features\nUI - User interface components\nAPI - API endpoints\n\nFound 3 prefixes";

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec list-prefixes",
    description: Some("List all prefixes"),
    output: Some(EXAMPLE_OUTPUT),
}];

const RELATED: &[&str] = &["create-prefix", "create-story", "create-bug", "create-task"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "list-prefixes",
    description: "List all registered work unit prefixes",
    usage: Some("fspec list-prefixes"),
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
