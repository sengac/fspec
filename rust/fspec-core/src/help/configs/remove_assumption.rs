//! `remove-assumption` help configuration (BUG-633).
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
    description: Some("Remove the assumption with ID 1"),
    command: "fspec remove-assumption AUTH-001 1",
    output: Some("✓ Removed assumption: \"Users have valid email addresses\""),
}];

const RELATED: &[&str] = &[
    "add-assumption",
    "restore-assumption",
    "show-work-unit",
    "show-deleted",
    "compact-work-unit",
];

const NOTES: &[&str] = &[
    "Uses stable IDs that never shift when items are removed",
    "A card's plain-text assumptions get stable IDs (their current indices) the first time one is changed",
    "Removing an already-removed assumption is idempotent (succeeds with message)",
    "compact-work-unit permanently drops removed assumptions and renumbers the rest",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "remove-assumption",
    description: "Remove an assumption by stable ID (soft delete with a deletedAt timestamp; restore it with restore-assumption)",
    usage: Some("fspec remove-assumption <workUnitId> <index>"),
    arguments: ARGS,
    options: OPTS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some("Use when an assumption no longer holds, e.g. a later answer superseded a provisional assumption."),
    when_not_to_use: None,
    prerequisites: &["Work unit must be in specifying status"],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: NOTES,
};
