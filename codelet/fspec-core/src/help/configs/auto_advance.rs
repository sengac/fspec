//! `auto-advance` help configuration — Rust port of
//! `src/commands/auto-advance-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js auto-advance --help`.

use super::super::{CommandExample, CommandHelpConfig, CommandOption};

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "--dry-run",
    description: "Show what would be advanced without making changes",
    default_value: None,
}];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec auto-advance",
        description: Some("Auto-advance eligible work units"),
        output: Some(
            "✓ AUTH-001: specifying → testing (all questions answered)\n✓ AUTH-002: testing → implementing (tests written)\n\nAdvanced 2 work units",
        ),
    },
    CommandExample {
        command: "fspec auto-advance --dry-run",
        description: Some("Preview auto-advance"),
        output: Some("Would advance: AUTH-001 (specifying → testing)"),
    },
];

const RELATED: &[&str] = &["update-work-unit-status", "board"];

const NOTES: &[&str] = &[
    "Checks ACDD workflow criteria for each state",
    "Safe to run multiple times (idempotent)",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "auto-advance",
    description: "Automatically advance work units through workflow when criteria are met",
    usage: Some("fspec auto-advance [options]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use to automatically move work units forward when they meet criteria (e.g., all questions answered → move to testing).",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: NOTES,
};
