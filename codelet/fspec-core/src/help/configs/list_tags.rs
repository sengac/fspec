//! `list-tags` help configuration — Rust port of
//! `src/commands/list-tags-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js list-tags --help`.

use super::super::{CommandExample, CommandHelpConfig, CommandOption};

const EXAMPLE_1_OUTPUT: &str = "Phase Tags:\n  @critical - Phase 1 features\n  @high - Phase 2 features\n\nComponent Tags:\n  @cli - CLI commands\n  @parser - Parser features";

const EXAMPLE_2_OUTPUT: &str = "@critical - Phase 1 features\n@high - Phase 2 features";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec list-tags",
        description: Some("List all tags"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec list-tags --category=\"Phase Tags\"",
        description: Some("List tags in specific category"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
];

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "--category <category>",
    description: "Filter tags by category (e.g., \"Phase Tags\", \"Component Tags\")",
    default_value: None,
}];

const RELATED: &[&str] = &["register-tag", "validate-tags", "tag-stats"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "list-tags",
    description: "List all registered tags from spec/tags.json",
    usage: Some("fspec list-tags [options]"),
    arguments: &[],
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
