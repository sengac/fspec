//! `show-deleted` help configuration — Rust port of
//! `src/commands/show-deleted-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js show-deleted --help`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig};

const ARGS: &[CommandArgument] = &[CommandArgument {
    name: "workUnitId",
    description: "Work unit ID",
    required: true,
}];

const EXAMPLE_OUTPUT: &str = "Deleted items for AUTH-001:\n\n[1] Old business rule (deleted: 2025-01-31T12:00:00.000Z)\n[3] Obsolete example (deleted: 2025-01-31T13:30:00.000Z)\n\nTotal: 2 deleted items";

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec show-deleted AUTH-001",
    description: Some("Show all deleted items with timestamps"),
    output: Some(EXAMPLE_OUTPUT),
}];

const NOTES: &[&str] = &[
    "Shows items with deleted: true flag",
    "Displays stable IDs that can be used with restore commands",
    "Shows deletedAt timestamps in ISO 8601 format",
    "Useful for debugging and selective restoration",
    "Returns empty list if no deleted items exist",
];

const RELATED: &[&str] = &[
    "restore-rule",
    "restore-example",
    "restore-question",
    "restore-architecture-note",
    "compact-work-unit",
    "show-work-unit",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "show-deleted",
    description: "Display all soft-deleted items with IDs, text, and deletedAt timestamps for debugging and selective restoration",
    usage: Some("fspec show-deleted <workUnitId>"),
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
    notes: NOTES,
};
