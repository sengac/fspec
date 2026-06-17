//! `restore-checkpoint` help configuration — Rust port of
//! `src/commands/restore-checkpoint-help.ts` (RPC-288).
//!
//! Byte-for-byte parity with `node dist/index.js restore-checkpoint --help`
//! piped to non-TTY (fixture:
//! `codelet/fspec/tests/fixtures/help/restore-checkpoint.txt`).
//!
//! Inherits the same three TS quirks documented on `list_checkpoints::CONFIG`
//! (comma-joined typical_workflow, doubled `fspec ` related-command prefix,
//! `Fix: undefined`).

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommonError};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "The ID of the work unit to restore a checkpoint for (e.g., AUTH-001, UI-002)",
        required: true,
    },
    CommandArgument {
        name: "checkpointName",
        description: "Name of the checkpoint to restore (e.g., \"baseline\", \"before-refactor\"). Use 'fspec list-checkpoints' to see available checkpoints.",
        required: true,
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec restore-checkpoint AUTH-001 baseline",
        description: Some("Restore after failed experiment"),
        output: Some("\u{2713} Restored checkpoint \"baseline\" for AUTH-001"),
    },
    CommandExample {
        command: "fspec restore-checkpoint UI-002 before-refactor",
        description: Some("Restore with dirty working directory (prompts for choice)"),
        output: Some("\u{26A0}\u{FE0F}  Working directory has uncommitted changes\n\nChoose how to proceed:\n  1. Commit changes first [Low risk]\n     Safest option. Commits current changes before restoration.\n  2. Stash changes and restore [Medium risk]\n     Temporarily saves changes. Can restore later, but may cause conflicts.\n  3. Force restore with merge [High risk]\n     Attempts to merge changes. May result in conflicts requiring manual resolution."),
    },
    CommandExample {
        command: "fspec restore-checkpoint AUTH-001 previous-state",
        description: Some("Restore with conflicts (AI receives system-reminder)"),
        output: Some("\u{2713} Restored checkpoint \"previous-state\" for AUTH-001\n\u{26A0}\u{FE0F}  Conflicts detected in 2 file(s):\n    - src/auth/login.ts\n    - src/auth/session.ts\n\n<system-reminder>\nCHECKPOINT CONFLICT RESOLUTION REQUIRED\nUse Read and Edit tools to resolve conflicts.\n</system-reminder>"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Work unit 'AUTH-001' does not exist",
        fix: "undefined",
    },
    CommonError {
        error: "Checkpoint 'baseline' not found for AUTH-001",
        fix: "undefined",
    },
    CommonError {
        error: "Merge conflicts detected",
        fix: "undefined",
    },
];

const NOTES: &[&str] = &[
    "Restoration uses 'git stash apply' (preserves stash for re-restoration)",
    "Same checkpoint can be restored multiple times",
    "Conflicts trigger AI-assisted resolution with system-reminders",
    "Tests should be run after conflict resolution (AI's responsibility)",
    "Interactive prompts explain risks when working directory is dirty",
    "Checkpoints are never automatically deleted during restoration",
];

const PREREQUISITES: &[&str] = &[
    "Work unit must exist",
    "Checkpoint must exist for this work unit",
    "Git repository must be initialized",
];

// TS quirk #2: doubled `fspec ` prefix.
const RELATED: &[&str] = &[
    "fspec checkpoint - Create manual checkpoint",
    "fspec list-checkpoints - List all checkpoints",
    "fspec cleanup-checkpoints - Delete old checkpoints",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "restore-checkpoint",
    description: "Restore a previously created checkpoint, applying saved file changes back to the working directory. Uses 'git stash apply' which preserves the checkpoint for re-restoration. If working directory is dirty, prompts for how to proceed. If conflicts occur, emits system-reminder for AI-assisted resolution.",
    usage: Some("fspec restore-checkpoint <workUnitId> <checkpointName>"),
    arguments: ARGS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use after experimental changes don't work out (revert to baseline), to try a different approach from same starting point, to recover from mistakes or failed changes, to restart from a known-good state, or when multiple experiments branch from same checkpoint.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    // TS quirk #1: comma-joined Array.toString()
    typical_workflow: Some(
        "Create baseline: fspec checkpoint AUTH-001 baseline,Try approach A: Make experimental changes,Doesn't work: fspec restore-checkpoint AUTH-001 baseline,Try approach B: Make different changes,Works: Keep approach B, checkpoint now exists for future experiments",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
