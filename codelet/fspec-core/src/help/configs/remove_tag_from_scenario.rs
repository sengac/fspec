//! `remove-tag-from-scenario` help configuration — Rust port of
//! `src/commands/remove-tag-from-scenario-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js remove-tag-from-scenario --help`
//! when piped to non-TTY. Fixture:
//! `codelet/fspec/tests/fixtures/help/remove-tag-from-scenario.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommonError,
};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "file",
        description: "Feature file path (e.g., spec/features/login.feature)",
        required: true,
    },
    CommandArgument {
        name: "scenario",
        description: "Scenario name exactly as it appears in the feature file (e.g., \"Login with valid credentials\")",
        required: true,
    },
    CommandArgument {
        name: "tags...",
        description: "One or more tags to remove (e.g., @wip @deprecated) - must include @ symbol",
        required: true,
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec remove-tag-from-scenario spec/features/login.feature \"Login with valid credentials\" @wip",
        description: Some("Remove single tag from scenario"),
        output: Some("✓ Removed tag @wip from scenario \"Login with valid credentials\""),
    },
    CommandExample {
        command: "fspec remove-tag-from-scenario spec/features/login.feature \"Login with valid credentials\" @wip @manual",
        description: Some("Remove multiple tags from scenario"),
        output: Some("✓ Removed tags @wip, @manual from scenario \"Login with valid credentials\""),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Scenario not found",
        fix: "Ensure scenario name matches exactly (case-sensitive)",
    },
    CommonError {
        error: "Tag not found on scenario",
        fix: "Use list-scenario-tags to see current tags on the scenario",
    },
];

const RELATED: &[&str] = &[
    "add-tag-to-scenario",
    "list-scenario-tags",
    "remove-tag-from-feature",
    "retag",
];

const NOTES: &[&str] = &[
    "Scenario names are case-sensitive and must match exactly",
    "Tags must include the @ symbol",
    "Removing a tag that does not exist on the scenario will show an error",
    "Feature file is automatically reformatted after tag removal",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "remove-tag-from-scenario",
    description: "Remove one or more tags from a specific scenario in a feature file",
    usage: Some("fspec remove-tag-from-scenario <file> <scenario> <tags...>"),
    arguments: ARGS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command when you need to remove tags from a specific scenario, such as removing @wip after completing work, removing @deprecated before deletion, or cleaning up incorrect tags.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
