//! `restore-assumption` help configuration (BUG-633).
//!
//! Feature: spec/features/remove-and-restore-assumptions.feature

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "Work unit ID",
        required: true,
    },
    CommandArgument {
        name: "index",
        description: "Assumption ID (stable index from show-work-unit or show-deleted)",
        required: true,
    },
];

const OPTS: &[CommandOption] = &[];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    description: Some("Restore the removed assumption with ID 1"),
    command: "fspec restore-assumption AUTH-001 1",
    output: Some("✓ Restored assumption: \"Users have valid email addresses\""),
}];

const RELATED: &[&str] = &[
    "remove-assumption",
    "add-assumption",
    "show-work-unit",
    "show-deleted",
    "compact-work-unit",
];

const NOTES: &[&str] = &[
    "Uses stable IDs that never shift when items are removed",
    "Restoring an already-active assumption is idempotent (succeeds with message)",
    "Non-existent IDs fail with a clear error message",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "restore-assumption",
    description: "Restore a removed assumption by stable ID (undeletes it and clears deletedAt)",
    usage: Some("fspec restore-assumption <workUnitId> <index>"),
    arguments: ARGS,
    options: OPTS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some("Use to bring back an assumption that was removed by mistake."),
    when_not_to_use: None,
    prerequisites: &["Work unit must be in specifying status"],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: NOTES,
};
