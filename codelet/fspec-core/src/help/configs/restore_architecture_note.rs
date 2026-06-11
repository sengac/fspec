//! `restore-architecture-note` help configuration — Rust port of the TS
//! Commander.js help output captured via
//! `node dist/index.js restore-architecture-note --help`.
//!
//! Mirrors `src/commands/restore-architecture-note-help.ts`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "Work unit ID",
        required: true,
    },
    CommandArgument {
        name: "index",
        description:
            "Architecture note ID (stable index from show-work-unit or show-deleted)",
        required: true,
    },
];

const OPTS: &[CommandOption] = &[CommandOption {
    flag: "--ids <ids>",
    description:
        "Restore multiple architecture notes with comma-separated IDs (e.g., \"2,5,7\")",
    default_value: None,
}];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        description: Some("Restore deleted architecture note with ID 2"),
        command: "fspec restore-architecture-note AUTH-001 2",
        output: Some(
            "✓ Restored architecture note from AUTH-001\nArchitecture note reappears at index [2] in show-work-unit",
        ),
    },
    CommandExample {
        description: Some("Restore multiple architecture notes at once (bulk restore)"),
        command: "fspec restore-architecture-note AUTH-001 --ids 2,5,7",
        output: Some("✓ Restored 3 architecture notes from AUTH-001"),
    },
];

const RELATED: &[&str] = &[
    "remove-architecture-note",
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
    name: "restore-architecture-note",
    description:
        "Restore soft-deleted architecture note by stable ID (undeletes item and clears deletedAt timestamp)",
    usage: Some("fspec restore-architecture-note <workUnitId> <index>"),
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
