//! `validate-work-units` help configuration — Rust port of
//! `src/commands/validate-work-units-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js validate-work-units --help`.
//! Captured fixture: `codelet/fspec/tests/fixtures/help/validate-work-units.txt`.

use super::super::{CommandExample, CommandHelpConfig, CommandOption};

const OPTS: &[CommandOption] = &[CommandOption {
    flag: "--fix",
    description: "Automatically fix issues where possible",
    default_value: None,
}];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec validate-work-units",
    description: Some("Validate all work units"),
    output: Some("✓ All work units valid\n  Checked 42 work units"),
}];

const RELATED: &[&str] = &["repair-work-units", "check"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "validate-work-units",
    description: "Validate work unit data integrity and relationships",
    usage: Some("fspec validate-work-units [options]"),
    arguments: &[],
    options: OPTS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use to check for data integrity issues like missing dependencies, invalid status transitions, or orphaned work units.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: &[],
};
