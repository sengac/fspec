//! `export-example-map` help configuration — Rust port of
//! `src/commands/export-example-map-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js export-example-map --help`.
//! Captured fixture: `rust/fspec/tests/fixtures/help/export-example-map.txt`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGUMENTS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "Work unit ID",
        required: true,
    },
    CommandArgument {
        name: "file",
        description: "Output JSON file path",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec export-example-map AUTH-001 example-map.json",
    description: Some("Export Example Mapping"),
    output: Some("✓ Exported Example Mapping to example-map.json"),
}];

const RELATED: &[&str] = &["import-example-map", "show-work-unit"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "export-example-map",
    description: "Export Example Mapping data from work unit to JSON file",
    usage: Some("fspec export-example-map <workUnitId> <file>"),
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
