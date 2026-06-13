//! `repair-work-units` help configuration — Rust port of
//! `src/commands/repair-work-units-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js repair-work-units --help`.

use super::super::{CommandExample, CommandHelpConfig, CommandOption};

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "--dry-run",
    description: "Show what would be repaired without making changes",
    default_value: None,
}];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec repair-work-units",
    description: Some("Repair all issues"),
    output: Some(
        "✓ Repaired 3 work units\n  - Fixed broken dependency: AUTH-001 → AUTH-999 (deleted)\n  - Reset invalid status: UI-002",
    ),
}];

const RELATED: &[&str] = &["validate-work-units"];

const NOTES: &[&str] = &[
    "Use --dry-run first to preview changes",
    "Creates backup before modifying",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "repair-work-units",
    description: "Repair work unit data integrity issues",
    usage: Some("fspec repair-work-units [options]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use when validate-work-units reports issues that need fixing, such as broken references or invalid data.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: NOTES,
};
