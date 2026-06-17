//! `checkpoint` help configuration — Rust port of
//! `src/commands/checkpoint-help.ts` (RPC-202).
//!
//! Byte-for-byte parity with `node dist/index.js checkpoint --help` piped to
//! non-TTY (captured fixture: `codelet/fspec/tests/fixtures/help/checkpoint.txt`).
//!
//! Inherits the same three TS quirks documented on `list_checkpoints::CONFIG`:
//!   1. `typical_workflow` is the JS Array's comma-joined `.toString()`; we
//!      pre-join with `,` so the verbatim Rust formatter matches.
//!   2. `related_commands` entries already include the literal `"fspec "`
//!      prefix and the formatter prefixes them AGAIN → `fspec fspec …`.
//!   3. `common_errors` TS items specify `solution:` but the formatter reads
//!      `err.fix` (`undefined`) → literal `"undefined"`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommonError};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "The ID of the work unit to create a checkpoint for (e.g., AUTH-001, UI-002)",
        required: true,
    },
    CommandArgument {
        name: "checkpointName",
        description: "User-provided name for the checkpoint (e.g., \"baseline\", \"before-refactor\"). Name should be descriptive of why you're creating this checkpoint.",
        required: true,
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec checkpoint AUTH-001 before-refactor",
        description: Some("Create checkpoint before refactoring"),
        output: Some("\u{2713} Created checkpoint \"before-refactor\" for AUTH-001\n  Captured 3 file(s)"),
    },
    CommandExample {
        command: "fspec checkpoint UI-002 baseline",
        description: Some("Create baseline for experiments"),
        output: Some("\u{2713} Created checkpoint \"baseline\" for UI-002\n  Captured 5 file(s)"),
    },
    CommandExample {
        command: "fspec checkpoint BUG-003 working-version",
        description: Some("Create checkpoint before risky change"),
        output: Some("\u{2713} Created checkpoint \"working-version\" for BUG-003\n  Captured 2 file(s)"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Work unit 'AUTH-001' does not exist",
        fix: "undefined",
    },
    CommonError {
        error: "Working directory is clean (no changes to checkpoint)",
        fix: "undefined",
    },
    CommonError {
        error: "Not a git repository",
        fix: "undefined",
    },
];

const NOTES: &[&str] = &[
    "Checkpoints capture ALL changes including untracked files (respecting .gitignore)",
    "Checkpoints persist until explicitly deleted (no automatic expiration)",
    "Manual checkpoints are marked with \u{1F4CC} emoji in listings",
    "Automatic checkpoints (from status changes) are marked with \u{1F916} emoji",
    "Checkpoint names should be descriptive (avoid generic names like \"temp\", \"test\")",
    "Use cleanup-checkpoints to manage checkpoint retention",
    "Checkpoints use git stash with format: fspec-checkpoint:{workUnitId}:{checkpointName}:{timestamp}",
];

const PREREQUISITES: &[&str] = &[
    "Work unit must exist (created with 'fspec create-story', 'fspec create-bug', or 'fspec create-task')",
    "Working directory should have uncommitted changes to capture",
    "Project must be a git repository",
];

// TS quirk #2: each entry already includes the "fspec " prefix; the formatter
// prefixes again, producing "fspec fspec …".
const RELATED_COMMANDS: &[&str] = &[
    "fspec restore-checkpoint - Restore a checkpoint",
    "fspec list-checkpoints - List all checkpoints",
    "fspec cleanup-checkpoints - Delete old checkpoints",
    "fspec update-work-unit-status - Auto-creates checkpoints",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "checkpoint",
    description: "Create a manual checkpoint for a work unit, capturing all file changes (including untracked files) as a git stash. Checkpoints enable safe experimentation by providing rollback points you can restore later.",
    usage: Some("fspec checkpoint <workUnitId> <checkpointName>"),
    arguments: ARGS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED_COMMANDS,
    when_to_use: Some(
        "Use checkpoints before trying experimental approaches or risky refactoring, creating a baseline before multiple experiments, saving progress before switching contexts, or before making changes you might want to revert.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    // TS quirk #1: comma-joined Array.toString()
    typical_workflow: Some(
        "Working on implementation: Make changes to code,Create checkpoint: fspec checkpoint AUTH-001 baseline,Try approach A: Make experimental changes,Doesn't work: fspec restore-checkpoint AUTH-001 baseline,Try approach B: Make different changes,Works: Continue with approach B,Cleanup: fspec cleanup-checkpoints AUTH-001 --keep-last 5",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
