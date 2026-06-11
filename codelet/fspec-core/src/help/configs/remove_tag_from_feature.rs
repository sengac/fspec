//! `remove-tag-from-feature` help configuration — Rust port of
//! `src/commands/remove-tag-from-feature-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js remove-tag-from-feature --help`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "file",
        description: "Feature file path",
        required: true,
    },
    CommandArgument {
        name: "tags...",
        description: "One or more tags to remove",
        required: true,
    },
];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec remove-tag-from-feature spec/features/login.feature @wip",
    description: Some("Remove tag"),
    output: Some("✓ Removed tag @wip from spec/features/login.feature"),
}];

const RELATED: &[&str] = &["add-tag-to-feature", "list-feature-tags"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "remove-tag-from-feature",
    description: "Remove one or more tags from a feature file",
    usage: Some("fspec remove-tag-from-feature <file> <tags...>"),
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
    notes: &[],
};
