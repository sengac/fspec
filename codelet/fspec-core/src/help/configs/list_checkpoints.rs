//! `list-checkpoints` help configuration — Rust port of
//! `src/commands/list-checkpoints-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js list-checkpoints --help`
//! piped to non-TTY (captured fixture:
//! `codelet/fspec/tests/fixtures/help/list-checkpoints.txt`).
//!
//! Inherits the same three TS quirks that `list_attachments::CONFIG`
//! documents:
//!   1. `typical_workflow` mirrors TS where the source is `string[]` but
//!      the help formatter renders `${config.typicalWorkflow}` — i.e. the
//!      JS Array's `.toString()` which is comma-joined. We pre-join with
//!      `,` here so the Rust formatter (which prints the field verbatim)
//!      produces the same line.
//!   2. `related_commands` entries in TS already include the literal
//!      `"fspec "` prefix (e.g. `"fspec checkpoint - Create manual
//!      checkpoint"`), and the help-formatter prefixes them AGAIN with
//!      `fspec ` — producing `fspec fspec checkpoint - ...`. We mirror
//!      this here.
//!   3. `common_errors` items in TS specify `solution:` but the formatter
//!      reads `err.fix` (which is `undefined`) producing `Fix: undefined`.
//!      We use the literal string `"undefined"` here.

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
        // TS quirk #3: solution → undefined under the help formatter.
        fix: "undefined",
    },
    CommonError {
        error: "No checkpoints found for <workUnitId>",
        fix: "undefined",
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
    "fspec checkpoint - Create manual checkpoint",
    "fspec restore-checkpoint - Restore a checkpoint",
    "fspec cleanup-checkpoints - Delete old checkpoints",
    "fspec update-work-unit-status - Auto-creates checkpoints",
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
        "Create checkpoints: fspec checkpoint AUTH-001 baseline,List checkpoints: fspec list-checkpoints AUTH-001,Choose checkpoint: See available names and timestamps,Restore: fspec restore-checkpoint AUTH-001 baseline,Or cleanup: fspec cleanup-checkpoints AUTH-001 --keep-last 5",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
