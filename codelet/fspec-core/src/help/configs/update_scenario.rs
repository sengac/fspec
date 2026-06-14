//! `update-scenario` help configuration — Rust port of
//! `src/commands/update-scenario-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js update-scenario --help`,
//! verified against `codelet/fspec/tests/fixtures/help/update-scenario.txt`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "file",
        description: "Feature file path",
        required: true,
    },
    CommandArgument {
        name: "old-name",
        description: "Current scenario name",
        required: true,
    },
    CommandArgument {
        name: "new-name",
        description: "New scenario name",
        required: true,
    },
];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec update-scenario spec/features/login.feature \"Old name\" \"New name\"",
    description: Some("Rename scenario and preserve coverage mappings"),
    output: Some("✓ Updated scenario name"),
}];

const RELATED: &[&str] = &["add-scenario", "delete-scenario"];

const NOTES: &[&str] = &[
    "Automatically renames scenario in .feature.coverage file if it exists",
    "All test mappings and implementation mappings are preserved during rename",
    "Use this command instead of manual delete + create to preserve coverage links",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "update-scenario",
    description: "Update a scenario name in a feature file",
    usage: Some("fspec update-scenario <file> <old-name> <new-name>"),
    arguments: ARGS,
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
