//! `validate-foundation-schema` help configuration — Rust port of
//! `src/commands/validate-foundation-schema-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js validate-foundation-schema --help`.

use super::super::{CommandExample, CommandHelpConfig};

const EXAMPLE_OUTPUT: &str =
    "✓ foundation.json is valid\n✓ All required fields present\n✓ All diagrams have valid structure";

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec validate-foundation-schema",
    description: Some("Validate foundation.json structure"),
    output: Some(EXAMPLE_OUTPUT),
}];

const RELATED: &[&str] = &["show-foundation-schema", "show-foundation", "update-foundation"];

const NOTES: &[&str] = &[
    "Uses Ajv for JSON Schema validation",
    "Checks required fields, types, and structure",
    "Run after any manual edits to foundation.json",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "validate-foundation-schema",
    description: "Validate foundation.json against its JSON schema using Ajv",
    usage: Some("fspec validate-foundation-schema"),
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
    notes: NOTES,
};
