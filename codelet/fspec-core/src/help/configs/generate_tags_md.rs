//! `generate-tags-md` help configuration — Rust port of the TS
//! `generate-tags-md` help registration.
//!
//! Byte-for-byte parity with `node dist/index.js generate-tags-md --help`
//! piped to non-TTY (see `codelet/fspec/tests/fixtures/help/generate-tags-md.txt`).

use super::super::{CommandExample, CommandHelpConfig};

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec generate-tags-md",
    description: Some("Generate human-readable tag documentation"),
    output: Some("✓ Generated spec/TAGS.md from tags.json\n✓ Tags grouped by category"),
}];

const RELATED: &[&str] = &["list-tags", "register-tag", "validate-tags"];

const NOTES: &[&str] = &[
    "TAGS.md is human-readable documentation",
    "tags.json is the machine-readable source of truth",
    "Generated file includes all registered tags grouped by category",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "generate-tags-md",
    description: "Generate spec/TAGS.md from tags.json",
    usage: Some("fspec generate-tags-md"),
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
