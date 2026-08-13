//! `add-dependencies` help configuration — Rust port of
//! `src/commands/add-dependencies-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js add-dependencies --help`.
//! Captured fixture: `rust/fspec/tests/fixtures/help/add-dependencies.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError, CommonPattern,
    CommonPatternEntry,
};

const ARGS: &[CommandArgument] = &[CommandArgument {
    name: "workUnitId",
    description: "Work unit ID to add dependencies to (e.g., AUTH-001)",
    required: true,
}];

const OPTS: &[CommandOption] = &[
    CommandOption {
        flag: "--blocks <ids...>",
        description: "Space-separated list of work unit IDs that this work unit blocks",
        default_value: None,
    },
    CommandOption {
        flag: "--blocked-by <ids...>",
        description: "Space-separated list of work unit IDs that block this work unit",
        default_value: None,
    },
    CommandOption {
        flag: "--depends-on <ids...>",
        description: "Space-separated list of work unit IDs this work unit depends on",
        default_value: None,
    },
    CommandOption {
        flag: "--relates-to <ids...>",
        description: "Space-separated list of related work unit IDs (non-blocking association)",
        default_value: None,
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command:
            "fspec add-dependencies AUTH-001 --depends-on AUTH-002 AUTH-003 --relates-to UI-001",
        description: Some("Add multiple dependencies and relations"),
        output: Some("✓ Added 3 dependencies successfully"),
    },
    CommandExample {
        command: "fspec add-dependencies API-001 --blocked-by DB-001 DB-002",
        description: Some("Mark work unit as blocked by multiple dependencies"),
        output: Some("✓ Added 2 dependencies successfully"),
    },
    CommandExample {
        command: "fspec add-dependencies UI-001 --blocks UI-002 UI-003 UI-004",
        description: Some("Mark work unit as blocking multiple other units"),
        output: Some("✓ Added 3 dependencies successfully"),
    },
    CommandExample {
        command: "fspec add-dependencies REFAC-001 --relates-to AUTH-001 API-001 DB-001",
        description: Some("Add related work units (non-blocking relationships)"),
        output: Some("✓ Added 3 dependencies successfully"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Error: Work unit AUTH-001 does not exist",
        fix: "Ensure work unit exists. Run: fspec list-work-units",
    },
    CommonError {
        error: "Error: Target work unit DB-001 does not exist",
        fix: "All target work units must exist before adding dependencies",
    },
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Setting Up Dependency Chain",
        example:
            "# API depends on DB schema and auth\nfspec add-dependencies API-001 --depends-on DB-001 AUTH-001\n\n# UI depends on API\nfspec add-dependencies UI-001 --depends-on API-001",
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Marking Blockers",
        example:
            "# Critical bug blocks multiple features\nfspec add-dependencies BUG-001 --blocks FEAT-001 FEAT-002 FEAT-003",
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Related Work (Non-Blocking)",
        example:
            "# Documentation relates to multiple features\nfspec add-dependencies DOC-001 --relates-to AUTH-001 API-001 UI-001",
        description: "undefined",
    }),
];

const RELATED: &[&str] = &[
    "add-dependency",
    "remove-dependency",
    "dependencies",
    "export-dependencies",
    "suggest-dependencies",
];

const NOTES: &[&str] = &[
    "All options accept space-separated lists of work unit IDs",
    "Each relationship is validated before being added",
    "Automatically creates bidirectional relationships where appropriate",
    "More efficient than multiple add-dependency calls",
    "Use --relates-to for associations that do not imply blocking or dependency",
];

const PREREQS: &[&str] = &[
    "Work unit must exist in spec/work-units.json",
    "Target work units must exist",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-dependencies",
    description: "Add multiple dependency relationships to a work unit at once",
    usage: Some("fspec add-dependencies <workUnitId> [options]"),
    arguments: ARGS,
    options: OPTS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command to add multiple dependency relationships to a work unit in a single command, instead of calling add-dependency multiple times. Efficient for setting up complex dependency graphs.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQS,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(
        "1. Create work units → 2. Identify dependencies → 3. fspec add-dependencies <id> --depends-on <deps> → 4. Verify: fspec dependencies <id>",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
