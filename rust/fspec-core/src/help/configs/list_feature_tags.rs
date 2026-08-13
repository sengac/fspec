//! `list-feature-tags` help configuration — Rust port of
//! `src/commands/list-feature-tags-help.ts`.
//!
//! The output of `format_command_help(&CONFIG)` is byte-for-byte identical
//! to `node dist/index.js list-feature-tags --help` when piped to non-TTY.
//!
//! Captured fixture: `rust/fspec/tests/fixtures/help/list-feature-tags.txt`

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const EXAMPLE_1_OUTPUT: &str = "@critical\n@authentication\n@cli\n@critical";

const EXAMPLE_2_OUTPUT: &str =
    "@critical (Phase Tags)\n@authentication (Feature Group Tags)\n@cli (Component Tags)\n@critical (Priority Tags)";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec list-feature-tags spec/features/login.feature",
        description: Some("List all tags on a feature file"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec list-feature-tags spec/features/login.feature --show-categories",
        description: Some("List tags with their categories"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
];

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "file",
    description: "Feature file path (e.g., spec/features/login.feature)",
    required: true,
}];

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "--show-categories",
    description: "Show tag categories from registry (spec/tags.json)",
    default_value: None,
}];

const RELATED: &[&str] = &[
    "add-tag-to-feature",
    "remove-tag-from-feature",
    "list-scenario-tags",
    "list-tags",
    "validate-tags",
];

const NOTES: &[&str] = &[
    "Shows only tags at the feature level (not scenario-level tags)",
    "Use --show-categories to see which category each tag belongs to",
    "Tags are displayed in the order they appear in the feature file",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "list-feature-tags",
    description: "List all tags on a specific feature file",
    usage: Some("fspec list-feature-tags <file> [options]"),
    arguments: ARGUMENTS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command when you need to see all tags applied to a feature file, either just the tag names or with their category information from the tag registry.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: NOTES,
};
