//! `add-tag-to-feature` help configuration — Rust port of
//! `src/commands/add-tag-to-feature-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js add-tag-to-feature --help`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "file",
        description: "Feature file path",
        required: true,
    },
    CommandArgument {
        name: "tags...",
        description: "One or more tags to add (e.g., @critical @critical)",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "--validate-registry",
    description: "Validate tags against spec/tags.json before adding",
    default_value: None,
}];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec add-tag-to-feature spec/features/login.feature @critical",
        description: Some("Add single tag"),
        output: Some("✓ Added tag @critical to spec/features/login.feature"),
    },
    CommandExample {
        command: "fspec add-tag-to-feature spec/features/login.feature @critical @critical",
        description: Some("Add multiple tags"),
        output: Some("✓ Added tags @critical, @critical to spec/features/login.feature"),
    },
    CommandExample {
        command:
            "fspec add-tag-to-feature spec/features/login.feature @custom-tag --validate-registry",
        description: Some("Add tag with registry validation"),
        output: Some(
            "Error: Tag @custom-tag is not registered in spec/tags.json\nRegister it first with: fspec register-tag @custom-tag \"Category\" \"Description\"",
        ),
    },
];

const RELATED: &[&str] = &[
    "remove-tag-from-feature",
    "list-feature-tags",
    "add-tag-to-scenario",
    "register-tag",
];

const NOTES: &[&str] = &[
    "Tags must be registered first (unless --validate-registry is skipped)",
    "Duplicates are automatically ignored",
    "Use --validate-registry to enforce tag registry compliance",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-tag-to-feature",
    description: "Add one or more tags to a feature file (feature-level tags)",
    usage: Some("fspec add-tag-to-feature <file> <tags...> [options]"),
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
    notes: NOTES,
};
