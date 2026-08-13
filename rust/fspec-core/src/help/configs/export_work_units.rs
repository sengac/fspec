//! `export-work-units` help configuration — Rust port of
//! `src/commands/export-work-units-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js export-work-units --help`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "format",
        description: "Output format: json or csv",
        required: true,
    },
    CommandArgument {
        name: "output",
        description: "Output file path",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--status <status>",
        description: "Filter by status",
        default_value: None,
    },
    CommandOption {
        flag: "--epic <epic>",
        description: "Filter by epic",
        default_value: None,
    },
];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec export-work-units json work-units.json",
    description: Some("Export to JSON"),
    output: Some("✓ Exported 42 work units to work-units.json"),
}];

const RELATED: &[&str] = &["list-work-units", "query-work-units"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "export-work-units",
    description: "Export work units to JSON or CSV format",
    usage: Some("fspec export-work-units <format> <output> [options]"),
    arguments: ARGS,
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
