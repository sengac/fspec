//! `list-features` help configuration — Rust port of
//! `src/commands/list-features-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js list-features --help`.

use super::super::{CommandExample, CommandHelpConfig, CommandOption};

const EXAMPLE_1_OUTPUT: &str =
    "spec/features/login.feature\nspec/features/signup.feature\n\nFound 2 features";

const EXAMPLE_2_OUTPUT: &str =
    "spec/features/login.feature [@critical @authentication]\n\nFound 1 feature";

const EXAMPLE_3_OUTPUT: &str =
    "spec/features/login.feature [@critical @critical @authentication]\n\nFound 1 feature";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec list-features",
        description: Some("List all features"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec list-features --tag=@critical",
        description: Some("List phase 1 features"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
    CommandExample {
        command: "fspec list-features --tag=@critical --tag=@critical",
        description: Some("List features with multiple tags"),
        output: Some(EXAMPLE_3_OUTPUT),
    },
];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--tag <tag>",
        description:
            "Filter by tag (can be used multiple times, e.g., --tag=@critical --tag=@critical)",
        default_value: None,
    },
    CommandOption {
        flag: "--format <format>",
        description: "Output format: table (default), json, or simple",
        default_value: None,
    },
];

const RELATED: &[&str] = &["show-feature", "create-feature", "list-tags", "validate"];

const NOTES: &[&str] = &[
    "Searches spec/features/ directory",
    "Multiple --tag flags filter with AND logic (feature must have all tags)",
    "JSON format useful for tooling integration",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "list-features",
    description: "List all feature files with optional filtering by tags",
    usage: Some("fspec list-features [options]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use to see all features in your project, or filter by specific tags to find features in a particular phase, component, or priority level.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: NOTES,
};
