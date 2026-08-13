//! `export-dependencies` help configuration — Rust port of
//! `src/commands/export-dependencies-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js export-dependencies --help`.
//! Captured fixture: `rust/fspec/tests/fixtures/help/export-dependencies.txt`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGUMENTS: &[CommandArgument] = &[
    CommandArgument {
        name: "format",
        description: "Output format: dot, mermaid, or json",
        required: true,
    },
    CommandArgument {
        name: "output",
        description: "Output file path",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec export-dependencies mermaid dependencies.mmd",
    description: Some("Export as Mermaid diagram"),
    output: Some("✓ Exported dependencies to dependencies.mmd"),
}];

const RELATED: &[&str] = &["add-dependency", "query-dependency-stats"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "export-dependencies",
    description: "Export dependency graph visualization in various formats",
    usage: Some("fspec export-dependencies <format> <output>"),
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
