//! `restore-example` help configuration — Rust port of
//! `src/commands/restore-example-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js restore-example --help`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "Work unit ID",
        required: true,
    },
    CommandArgument {
        name: "index",
        description: "Example ID (stable index from show-work-unit or show-deleted)",
        required: true,
    },
];

const OPTS: &[CommandOption] = &[];

const EXAMPLE_1_OUTPUT: &str =
    "✓ Restored example from AUTH-001\nExample reappears at index [2] in show-work-unit";

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec restore-example AUTH-001 2",
    description: Some("Restore deleted example with ID 2"),
    output: Some(EXAMPLE_1_OUTPUT),
}];

const RELATED: &[&str] = &[
    "remove-example",
    "show-work-unit",
    "show-deleted",
    "compact-work-unit",
];

const NOTES: &[&str] = &[
    "Uses stable IDs that never shift when items are removed",
    "Restoring an already-active item is idempotent (succeeds with message)",
    "Non-existent IDs will fail with clear error message",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "restore-example",
    description:
        "Restore soft-deleted example by stable ID (undeletes item and clears deletedAt timestamp)",
    usage: Some("fspec restore-example <workUnitId> <index>"),
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
