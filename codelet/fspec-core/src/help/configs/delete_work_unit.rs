//! `delete-work-unit` help configuration — Rust port of
//! `src/commands/delete-work-unit-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js delete-work-unit --help`.
//!
//! NOTE: the two COMMON PATTERNS entries in the TS source carry no
//! `description` field, so the TS help formatter interpolates the literal
//! string `undefined` on the description line. We reproduce that verbatim
//! by setting `description: "undefined"` on each structured pattern — the
//! captured fixture asserts this byte-for-byte.

use super::super::{
    CommandArgument, CommonError, CommandExample, CommandHelpConfig, CommandOption, CommonPattern,
    CommonPatternEntry,
};

const ARGS: &[CommandArgument] = &[CommandArgument {
    name: "workUnitId",
    description: "Work unit ID to delete (e.g., AUTH-001)",
    required: true,
}];

const OPTS: &[CommandOption] = &[
    CommandOption {
        flag: "--force",
        description: "Force deletion without validation checks",
        default_value: None,
    },
    CommandOption {
        flag: "--skip-confirmation",
        description: "Skip confirmation prompt (use in scripts)",
        default_value: None,
    },
    CommandOption {
        flag: "--cascade-dependencies",
        description: "Remove all dependency relationships before deleting",
        default_value: None,
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec delete-work-unit AUTH-999",
        description: Some("Delete work unit (with confirmation)"),
        output: Some("Are you sure? (y/N): y\n✓ Work unit AUTH-999 deleted successfully"),
    },
    CommandExample {
        command: "fspec delete-work-unit AUTH-999 --skip-confirmation",
        description: Some("Delete without confirmation"),
        output: Some("✓ Work unit AUTH-999 deleted successfully"),
    },
    CommandExample {
        command: "fspec delete-work-unit AUTH-999 --cascade-dependencies",
        description: Some("Delete and remove all dependencies"),
        output: Some(
            "✓ Work unit AUTH-999 deleted successfully\n⚠ This work unit blocks 2 work unit(s): API-001, UI-001",
        ),
    },
    CommandExample {
        command: "fspec delete-work-unit AUTH-999 --force --skip-confirmation",
        description: Some("Force delete without any checks"),
        output: Some("✓ Work unit AUTH-999 deleted successfully"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Error: Work unit AUTH-999 does not exist",
        fix: "Verify work unit ID with: fspec list-work-units",
    },
    CommonError {
        error: "Error: Cannot delete work unit with children: AUTH-002, AUTH-003. Delete children first or remove parent relationship.",
        fix: "Either delete child work units first, or remove the parent relationship from children",
    },
    CommonError {
        error: "Error: Work unit AUTH-001 has dependencies. Use --cascade-dependencies flag to remove dependencies and delete.",
        fix: "Add --cascade-dependencies flag to remove all dependency relationships before deletion",
    },
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Safe Deletion with Checks",
        example: "# Check what will be deleted\nfspec show-work-unit AUTH-999\nfspec dependencies AUTH-999\n\n# Delete with cascading\nfspec delete-work-unit AUTH-999 --cascade-dependencies",
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Scripted Deletion",
        example: "# Delete in automated scripts\nfspec delete-work-unit AUTH-999 --skip-confirmation --cascade-dependencies",
        description: "undefined",
    }),
];

const RELATED: &[&str] = &[
    "list-work-units",
    "create-story",
    "create-bug",
    "create-task",
    "show-work-unit",
    "dependencies",
];

const NOTES: &[&str] = &[
    "Deletion is permanent and cannot be undone",
    "Deletes all Example Mapping data associated with the work unit",
    "Removes work unit from all state arrays (backlog, specifying, etc.)",
    "Removes work unit from parent's children array if it has a parent",
    "Cannot delete work units that have children (delete children first)",
    "Use --cascade-dependencies to remove all dependency relationships automatically",
    "--force flag bypasses validation checks (use with caution)",
    "--skip-confirmation is useful for automated scripts",
];

const PREREQUISITES: &[&str] = &["Work unit must exist in spec/work-units.json"];

const WHEN_TO_USE: &str = "Use this command to permanently remove a work unit from the project. This deletes all associated data including Example Mapping results, dependencies, and state tracking. Use with caution as this operation cannot be undone.";

const TYPICAL_WORKFLOW: &str = "1. Verify work unit: fspec show-work-unit <id> → 2. Check dependencies: fspec dependencies <id> → 3. Delete: fspec delete-work-unit <id> --cascade-dependencies";

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "delete-work-unit",
    description: "Delete a work unit and all its data",
    usage: Some("fspec delete-work-unit <workUnitId> [options]"),
    arguments: ARGS,
    options: OPTS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(WHEN_TO_USE),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(TYPICAL_WORKFLOW),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
