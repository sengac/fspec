//! `delete-tag` help configuration — Rust port of the TS Commander.js
//! help output captured via `node dist/index.js delete-tag --help`.
//!
//! Captured fixture: `codelet/fspec/tests/fixtures/help/delete-tag.txt`.
//! Both the dispatcher path and the standalone fspec Rust binary serve
//! help via `format_command_help(&CONFIG)` so the rendered output is
//! byte-identical to the TS reference.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "tag",
    description: "Tag to delete",
    required: true,
}];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--force",
        description: "Skip confirmation",
        default_value: None,
    },
    CommandOption {
        flag: "--dry-run",
        description: "Show what would be deleted without making changes",
        default_value: None,
    },
];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    description: Some("Delete tag"),
    command: "fspec delete-tag @deprecated",
    output: Some("✓ Deleted tag @deprecated"),
}];

const RELATED: &[&str] = &["register-tag", "list-tags"];

const NOTES: &[&str] = &[
    "Tag must not be in use in any feature files",
    "Run validate-tags after deletion to verify",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "delete-tag",
    description: "Delete a tag from the registry",
    usage: Some("fspec delete-tag <tag> [options]"),
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
    notes: NOTES,
};
