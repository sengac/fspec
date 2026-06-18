//! `update-work-unit-status` help configuration — Rust port of
//! `src/commands/update-work-unit-status-help.ts` (RPC-319).
//!
//! Byte-for-byte parity with `node dist/index.js update-work-unit-status
//! --help` piped to non-TTY.
//!
//! ## Parity quirk: `undefined` option flags
//!
//! The TS help config (`update-work-unit-status-help.ts`) declares each option
//! with a `name` field (`--reason <text>`, `--blocked-reason <text>`,
//! `--skip-temporal-validation`) but the shared `formatCommandHelp` formatter
//! reads `opt.flag` — a field the config never sets. The result is that the TS
//! `--help` literally prints `undefined` for each option flag line. To match
//! the captured fixture byte-for-byte we set `flag: "undefined"` for all three
//! options. (The description lines are still rendered from `opt.description`.)

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "id",
        description: "Work unit ID (e.g., AUTH-001)",
        required: true,
    },
    CommandArgument {
        name: "status",
        description:
            "New status: backlog, specifying, testing, implementing, validating, done, blocked",
        required: true,
    },
];

// See module docs: the TS config uses `name` (not `flag`), so the formatter's
// `opt.flag` read yields the literal string `undefined`. We reproduce that.
const OPTS: &[CommandOption] = &[
    CommandOption {
        flag: "undefined",
        description: "Reason for status change (optional, added to state history)",
        default_value: None,
    },
    CommandOption {
        flag: "undefined",
        description: "Reason for blocked status (required when status is \"blocked\")",
        default_value: None,
    },
    CommandOption {
        flag: "undefined",
        description:
            "Skip temporal ordering validation (for reverse ACDD or importing existing work)",
        default_value: None,
    },
];

const PREREQUISITES: &[&str] = &[
    "Work unit must exist (create with fspec create-story, create-bug, or create-task)",
    "For transition to testing: All Example Mapping questions must be answered",
    "For transition to implementing: Tests must be written",
    "For transition to active states: All blocking dependencies must be completed (status: done)",
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec update-work-unit-status AUTH-001 specifying",
        description: Some("Move to specifying (start Example Mapping and write feature files)"),
        output: Some("✓ Work unit AUTH-001 status updated to specifying"),
    },
    CommandExample {
        command:
            "fspec update-work-unit-status AUTH-001 blocked --blocked-reason=\"Waiting for API design\"",
        description: Some("Mark work unit as blocked with reason"),
        output: Some("✓ Work unit AUTH-001 status updated to blocked"),
    },
    CommandExample {
        command: "fspec update-work-unit-status LEGACY-001 testing --skip-temporal-validation",
        description: Some("Import existing work and skip temporal validation"),
        output: Some("✓ Work unit LEGACY-001 status updated to testing"),
    },
    CommandExample {
        command: "fspec update-work-unit-status UI-001 implementing",
        description: Some("Attempt to start work on blocked work unit (will fail)"),
        output: Some(
            "✗ Cannot start work on UI-001: work unit is blocked by incomplete dependencies.\n\nActive blockers:\n  - AUTH-001 (status: implementing)\n\nComplete blocking work units or remove dependencies before starting work.",
        ),
    },
];

const RELATED: &[&str] = &["show-work-unit", "list-work-units", "board", "auto-advance"];

const NOTES: &[&str] = &[
    "ACDD enforces strict workflow: you cannot skip states",
    "FEAT-011: Temporal ordering is enforced - files must be created AFTER entering their required state",
    "Moving to testing: feature files must be created AFTER entering specifying state",
    "Moving to implementing: test files must be created AFTER entering testing state",
    "Use --skip-temporal-validation for reverse ACDD or importing existing work",
    "Status \"blocked\" can be used from any state when progress is prevented",
    "Moving to testing requires all Example Mapping questions to be answered first",
    "Cannot move to active states (specifying, testing, implementing, validating) if work unit has incomplete blocking dependencies",
    "Use \"fspec remove-dependency\" to remove blockers or complete blocking work units first",
    "Backward movement is allowed: can move from implementing → testing → specifying when mistakes discovered",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "update-work-unit-status",
    description: "Move a work unit through the ACDD workflow by updating its status",
    usage: Some("fspec update-work-unit-status <id> <status>"),
    arguments: ARGS,
    options: OPTS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command when moving work through ACDD workflow stages: from backlog to specifying (writing feature files), to testing (writing tests), to implementing (writing code), to validating (quality checks), and finally to done.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    typical_workflow: Some(
        "backlog → specifying (Example Mapping + feature file) → testing (write failing tests) → implementing (make tests pass) → validating (run all tests + quality checks) → done",
    ),
    common_errors: &[],
    notes: NOTES,
};
