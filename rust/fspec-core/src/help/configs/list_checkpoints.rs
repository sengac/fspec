//! `list-checkpoints` help configuration — Rust port of
//! `src/commands/list-checkpoints-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js list-checkpoints --help`
//! piped to non-TTY (captured fixture:
//! `rust/fspec/tests/fixtures/help/list-checkpoints.txt`).
//!
//! BUG-566: the TypeScript formatter printed `undefined` for keys this config
//! left unset; the config now holds the real text (each fix is the TS
//! `solution`, a pattern without a description prints none).

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommonError};

const ARGS: &[CommandArgument] = &[CommandArgument {
    name: "workUnitId",
    description: "The ID of the work unit to list checkpoints for (e.g., AUTH-001, UI-002)",
    required: true,
}];

const EXAMPLE_1_OUTPUT: &str = "Checkpoints for AUTH-001:\n\n📌  before-refactor (manual)\n   Created: 2025-10-21T14:30:00.000Z\n\n📌  baseline (manual)\n   Created: 2025-10-21T13:15:00.000Z\n\n🤖  AUTH-001-auto-testing (automatic)\n   Created: 2025-10-21T10:00:00.000Z";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec list-checkpoints AUTH-001",
        description: Some("List all checkpoints for work unit"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec list-checkpoints UI-002",
        description: Some("List checkpoints for work unit with no checkpoints"),
        output: Some("No checkpoints found for UI-002"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Work unit 'AUTH-001' does not exist",
        fix: "Create the work unit first with 'fspec create-story', 'fspec create-bug', or 'fspec create-task'",
    },
    CommonError {
        error: "No checkpoints found for <workUnitId>",
        fix: "Not an error - work unit has no checkpoints yet. Create a checkpoint with 'fspec checkpoint' or transition status.",
    },
];

const NOTES: &[&str] = &[
    "All checkpoints are shown by default (both automatic and manual)",
    "Checkpoints are sorted by creation time (newest first)",
    "Automatic checkpoints use naming pattern: {workUnitId}-auto-{state}",
    "Manual checkpoints use user-provided names",
    "No pagination - all checkpoints for work unit are displayed",
    "Empty list is not an error - just means no checkpoints exist yet",
];

// TS quirk #2: each entry already includes "fspec " prefix; the formatter
// prefixes again, producing "fspec fspec …".
const RELATED: &[&str] = &[
    "checkpoint - Create manual checkpoint",
    "restore-checkpoint - Restore a checkpoint",
    "cleanup-checkpoints - Delete old checkpoints",
    "update-work-unit-status - Auto-creates checkpoints",
];

const PREREQUISITES: &[&str] = &["Work unit must exist", "Git repository must be initialized"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "list-checkpoints",
    description: "List all checkpoints for a work unit, showing both automatic and manual checkpoints with visual indicators (🤖 automatic, 📌 manual). Displays checkpoint names, types, and creation timestamps.",
    usage: Some("fspec list-checkpoints <workUnitId>"),
    arguments: ARGS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use before restoring a checkpoint to see what's available, to verify a checkpoint was created successfully, to see the history of checkpoints for a work unit, to decide which old checkpoints to cleanup, or to understand automatic vs manual checkpoint patterns.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    // TS quirk #1: comma-joined Array.toString()
    typical_workflow: Some(
        "Create checkpoints: fspec checkpoint AUTH-001 baseline. List checkpoints: fspec list-checkpoints AUTH-001. Choose checkpoint: See available names and timestamps. Restore: fspec restore-checkpoint AUTH-001 baseline. Or cleanup: fspec cleanup-checkpoints AUTH-001 --keep-last 5",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
