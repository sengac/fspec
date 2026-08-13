//! `add-tag-to-scenario` help configuration — Rust port of
//! `src/commands/add-tag-to-scenario-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js add-tag-to-scenario --help`
//! when piped to non-TTY. Fixture: `rust/fspec/tests/fixtures/help/add-tag-to-scenario.txt`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "file",
        description: "Feature file path",
        required: true,
    },
    CommandArgument {
        name: "scenario",
        description: "Scenario name (must match exactly)",
        required: true,
    },
    CommandArgument {
        name: "tags...",
        description: "One or more tags to add",
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
        command: "fspec add-tag-to-scenario spec/features/login.feature \"Login with invalid password\" @edge-case",
        description: Some("Add tag to scenario"),
        output: Some("✓ Added tag @edge-case to scenario"),
    },
    CommandExample {
        command: "fspec add-tag-to-scenario spec/features/login.feature \"Login scenario\" @WORK-UNIT-001 --validate-registry",
        description: Some("Add tag with registry validation"),
        output: Some("✓ Added tag @WORK-UNIT-001 to scenario \"Login scenario\""),
    },
];

const RELATED: &[&str] = &[
    "remove-tag-from-scenario",
    "list-scenario-tags",
    "register-tag",
];

const NOTES: &[&str] = &[
    "Scenario name is case-sensitive",
    "Use --validate-registry to enforce tag registry compliance",
    "Duplicates are automatically ignored",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-tag-to-scenario",
    description: "Add one or more tags to a specific scenario",
    usage: Some("fspec add-tag-to-scenario <file> <scenario> <tags...> [options]"),
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
