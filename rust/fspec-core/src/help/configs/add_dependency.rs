//! `add-dependency` help configuration — Rust port of
//! `src/commands/add-dependency-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js add-dependency --help`.
//! Captured fixture: `rust/fspec/tests/fixtures/help/add-dependency.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError,
};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "id",
        description: "Work unit ID to add dependency to",
        required: true,
    },
    CommandArgument {
        name: "dependsOnId",
        description: "Work unit ID that this depends on (shorthand for --depends-on)",
        required: false,
    },
];

const OPTS: &[CommandOption] = &[
    CommandOption {
        flag: "--blocks <id>",
        description: "This work unit blocks the specified work unit",
        default_value: None,
    },
    CommandOption {
        flag: "--blocked-by <id>",
        description: "This work unit is blocked by the specified work unit",
        default_value: None,
    },
    CommandOption {
        flag: "--depends-on <id>",
        description: "This work unit depends on the specified work unit",
        default_value: None,
    },
    CommandOption {
        flag: "--relates-to <id>",
        description: "This work unit is related to the specified work unit (no blocking)",
        default_value: None,
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec add-dependency AUTH-002 AUTH-001",
        description: Some("Shorthand: AUTH-002 depends on AUTH-001"),
        output: Some("✓ Added dependency: AUTH-002 depends on AUTH-001"),
    },
    CommandExample {
        command: "fspec add-dependency AUTH-002 --blocks API-001",
        description: Some("AUTH-002 blocks API-001 from starting"),
        output: Some("✓ Added dependency: AUTH-002 blocks API-001"),
    },
    CommandExample {
        command: "fspec add-dependency UI-001 --blocked-by API-001",
        description: Some("UI-001 is blocked by API-001"),
        output: Some("✓ Added dependency: UI-001 blocked by API-001"),
    },
    CommandExample {
        command: "fspec add-dependency DASH-001 --depends-on AUTH-001",
        description: Some("Explicit: DASH-001 depends on AUTH-001"),
        output: Some("✓ Added dependency: DASH-001 depends on AUTH-001"),
    },
    CommandExample {
        command: "fspec add-dependency UI-005 --relates-to UI-004",
        description: Some("UI-005 is related to UI-004 (no blocking)"),
        output: Some("✓ Added dependency: UI-005 relates to UI-004"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Work unit AUTH-999 not found",
        fix: "Verify the work unit ID exists with: fspec list-work-units",
    },
    CommonError {
        error: "Circular dependency detected",
        fix: "Remove the circular dependency chain before adding this relationship",
    },
];

const RELATED: &[&str] = &[
    "remove-dependency",
    "dependencies",
    "export-dependencies",
    "clear-dependencies",
];

const NOTES: &[&str] = &[
    "Use shorthand syntax (two arguments) for simple depends-on relationships",
    "Use explicit flags (--blocks, --depends-on) for clarity in complex dependency graphs",
    "Circular dependencies are not allowed and will be rejected",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-dependency",
    description:
        "Add dependency relationships between work units to track blockers and dependencies",
    usage: Some("fspec add-dependency <id> [dependsOnId] [options]"),
    arguments: ARGS,
    options: OPTS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: None,
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
