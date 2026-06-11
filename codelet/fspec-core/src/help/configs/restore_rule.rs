//! `restore-rule` help configuration — Rust port of
//! `src/commands/restore-rule-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js restore-rule --help`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "Work unit ID",
        required: true,
    },
    CommandArgument {
        name: "index",
        description: "Rule ID (stable index from show-work-unit or show-deleted)",
        required: true,
    },
];

const OPTS: &[CommandOption] = &[CommandOption {
    flag: "--ids <ids>",
    description: "Restore multiple rules with comma-separated IDs (e.g., \"2,5,7\")",
    default_value: None,
}];

const EXAMPLE_1_OUTPUT: &str =
    "✓ Restored rule from AUTH-001\nRule reappears at index [2] in show-work-unit";
const EXAMPLE_2_OUTPUT: &str = "✓ Restored 3 rules from AUTH-001";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec restore-rule AUTH-001 2",
        description: Some("Restore deleted rule with ID 2"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec restore-rule AUTH-001 --ids 2,5,7",
        description: Some("Restore multiple rules at once (bulk restore)"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
];

const RELATED: &[&str] = &[
    "remove-rule",
    "show-work-unit",
    "show-deleted",
    "compact-work-unit",
];

const NOTES: &[&str] = &[
    "Uses stable IDs that never shift when items are removed",
    "Restoring an already-active item is idempotent (succeeds with message)",
    "Non-existent IDs will fail with clear error message",
    "Bulk restore validates all IDs before restoring any item",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "restore-rule",
    description:
        "Restore soft-deleted business rule by stable ID (undeletes item and clears deletedAt timestamp)",
    usage: Some("fspec restore-rule <workUnitId> <index>"),
    arguments: ARGS,
    options: OPTS,
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
