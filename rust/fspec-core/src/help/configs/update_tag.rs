//! `update-tag` help configuration — Rust port of
//! `src/commands/update-tag-help.ts`. Captured fixture lives at
//! `rust/fspec/tests/fixtures/help/update-tag.txt`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "tag",
    description: "Tag to update",
    required: true,
}];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--description <description>",
        description: "New description",
        default_value: None,
    },
    CommandOption {
        flag: "--category <category>",
        description: "New category",
        default_value: None,
    },
];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    description: Some("Update tag description"),
    command: "fspec update-tag @performance --description \"High-performance features\"",
    output: Some("✓ Updated tag @performance"),
}];

const RELATED: &[&str] = &["register-tag", "delete-tag", "list-tags"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "update-tag",
    description: "Update a registered tag description or category",
    usage: Some("fspec update-tag <tag> [options]"),
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
