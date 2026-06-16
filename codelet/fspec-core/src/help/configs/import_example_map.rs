//! `import-example-map` help configuration — Rust port of
//! `src/commands/import-example-map-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js import-example-map --help`.
//! Captured fixture: `codelet/fspec/tests/fixtures/help/import-example-map.txt`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGUMENTS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "Work unit ID",
        required: true,
    },
    CommandArgument {
        name: "file",
        description: "JSON file path containing Example Mapping data",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec import-example-map AUTH-001 example-map.json",
    description: Some("Import Example Mapping"),
    output: Some("✓ Imported Example Mapping data to AUTH-001\n  3 rules, 5 examples, 2 questions"),
}];

const RELATED: &[&str] = &["export-example-map", "show-work-unit"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "import-example-map",
    description: "Import Example Mapping data from JSON file to work unit",
    usage: Some("fspec import-example-map <workUnitId> <file>"),
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
