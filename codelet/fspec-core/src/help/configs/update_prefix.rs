//! `update-prefix` help configuration — Rust port of
//! `src/commands/update-prefix-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js update-prefix --help`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "prefix",
    description: "Prefix to update",
    required: true,
}];

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "--description <description>",
    description: "New description",
    default_value: None,
}];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec update-prefix AUTH --description \"Authentication and authorization\"",
    description: Some("Update prefix description"),
    output: Some("✓ Updated prefix AUTH"),
}];

const RELATED: &[&str] = &["create-prefix", "list-prefixes"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "update-prefix",
    description: "Update the description of a work unit prefix",
    usage: Some("fspec update-prefix <prefix> [options]"),
    arguments: ARGUMENTS,
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
