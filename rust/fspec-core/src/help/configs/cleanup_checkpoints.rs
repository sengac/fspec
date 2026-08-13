//! `cleanup-checkpoints` help configuration — Rust port of
//! `src/commands/cleanup-checkpoints-help.ts` (RPC-203).
//!
//! Byte-for-byte parity with `node dist/index.js cleanup-checkpoints --help`
//! piped to non-TTY (fixture:
//! `rust/fspec/tests/fixtures/help/cleanup-checkpoints.txt`).
//!
//! Inherits the same three TS quirks documented on `list_checkpoints::CONFIG`
//! (comma-joined typical_workflow, doubled `fspec ` related-command prefix,
//! `Fix: undefined`).

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError,
};

const ARGS: &[CommandArgument] = &[CommandArgument {
    name: "workUnitId",
    description: "The ID of the work unit to cleanup checkpoints for (e.g., AUTH-001, UI-002)",
    required: true,
}];

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "--keep-last <N>",
    description: "Number of most recent checkpoints to preserve (required). Older checkpoints beyond this count will be deleted.",
    default_value: None,
}];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec cleanup-checkpoints AUTH-001 --keep-last 5",
        description: Some("Keep last 5 checkpoints, delete rest"),
        output: Some("Cleaning up checkpoints for AUTH-001 (keeping last 5)...\n\nDeleted 7 checkpoint(s):\n  - checkpoint-1 (2025-10-20T10:00:00.000Z)\n  - checkpoint-2 (2025-10-20T11:00:00.000Z)\n  ...\n\nPreserved 5 checkpoint(s):\n  - current-state (2025-10-21T14:30:00.000Z)\n  - after-optimization (2025-10-21T13:15:00.000Z)\n  ...\n\n\u{2713} Cleanup complete: 7 deleted, 5 preserved"),
    },
    CommandExample {
        command: "fspec cleanup-checkpoints UI-002 --keep-last 1",
        description: Some("Keep only last checkpoint"),
        output: Some("Cleaning up checkpoints for UI-002 (keeping last 1)...\n\nDeleted 4 checkpoint(s)\nPreserved 1 checkpoint(s)\n\n\u{2713} Cleanup complete: 4 deleted, 1 preserved"),
    },
    CommandExample {
        command: "fspec cleanup-checkpoints BUG-003 --keep-last 10",
        description: Some("No cleanup needed (already have \u{2264} N checkpoints)"),
        output: Some("Checkpoints for BUG-003: 3 total (\u{2264} 10)\nNo cleanup needed - all checkpoints preserved"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Work unit 'AUTH-001' does not exist",
        fix: "undefined",
    },
    CommonError {
        error: "No checkpoints found for AUTH-001",
        fix: "undefined",
    },
    CommonError {
        error: "--keep-last option is required",
        fix: "undefined",
    },
];

const NOTES: &[&str] = &[
    "Deletion is permanent - checkpoints cannot be recovered after cleanup",
    "Checkpoints are deleted oldest-first based on creation timestamp",
    "Both automatic and manual checkpoints are included in cleanup",
    "No dry-run option - deletion happens immediately",
    "Preserves exactly N most recent checkpoints (not N+1)",
    "If work unit has \u{2264} N checkpoints, no deletion occurs",
];

const PREREQUISITES: &[&str] = &[
    "Work unit must exist",
    "At least one checkpoint must exist for the work unit",
    "Git repository must be initialized",
];

// TS quirk #2: doubled `fspec ` prefix.
const RELATED: &[&str] = &[
    "fspec checkpoint - Create manual checkpoint",
    "fspec restore-checkpoint - Restore a checkpoint",
    "fspec list-checkpoints - List all checkpoints",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "cleanup-checkpoints",
    description: "Delete old checkpoints for a work unit while preserving the most recent N checkpoints. Helps manage checkpoint retention and avoid accumulating too many old save points. Deletion is based on creation timestamp (oldest checkpoints deleted first).",
    usage: Some("fspec cleanup-checkpoints <workUnitId> --keep-last <N>"),
    arguments: ARGS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use after many experiments leave too many checkpoints, to manage checkpoint retention periodically, when you're done with a work unit and want to cleanup, to keep only recent checkpoints and remove old ones, or as part of work unit completion workflow.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    // TS quirk #1: comma-joined Array.toString()
    typical_workflow: Some(
        "Create many checkpoints during experimentation,List checkpoints: fspec list-checkpoints AUTH-001,Decide how many to keep (e.g., last 5),Cleanup: fspec cleanup-checkpoints AUTH-001 --keep-last 5,Verify: fspec list-checkpoints AUTH-001",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
