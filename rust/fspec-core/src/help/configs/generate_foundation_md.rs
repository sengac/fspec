//! `generate-foundation-md` help configuration — Rust port of
//! `src/commands/generate-foundation-md-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js generate-foundation-md
//! --help` piped to non-TTY.

use super::super::{CommandExample, CommandHelpConfig};

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec generate-foundation-md",
    description: Some("Generate human-readable foundation documentation"),
    output: Some(
        "✓ Generated spec/FOUNDATION.md from foundation.json\n✓ Mermaid diagrams included",
    ),
}];

const RELATED: &[&str] = &[
    "show-foundation",
    "update-foundation",
    "validate-foundation-schema",
];

const NOTES: &[&str] = &[
    "FOUNDATION.md is human-readable documentation",
    "foundation.json is the machine-readable source of truth",
    "Generated file includes project metadata and Mermaid diagrams",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "generate-foundation-md",
    description: "Generate spec/FOUNDATION.md from foundation.json",
    usage: Some("fspec generate-foundation-md"),
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
