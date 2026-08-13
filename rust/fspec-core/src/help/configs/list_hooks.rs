//! `list-hooks` help configuration — Rust port of
//! `src/commands/list-hooks-help.ts`.
//!
//! The output of `format_command_help(&CONFIG)` is byte-for-byte identical
//! to `node dist/index.js list-hooks --help` when piped to a non-TTY.

use super::super::{CommandExample, CommandHelpConfig, CommonError};

const EXAMPLE_1_OUTPUT: &str = "Configured Hooks:\n\npre-update-work-unit-status:\n  - validate-feature-file\n  - check-blockers\n\npost-implementing:\n  - run-tests\n  - lint-code\n\npost-validating:\n  - notify-slack";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec list-hooks",
        description: Some("List all configured hooks"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec list-hooks",
        description: Some("When no hooks are configured"),
        output: Some("No hooks are configured"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[CommonError {
    error: "No hooks are configured",
    fix: "This is not an error - it means you have no hooks configured yet. Create spec/fspec-hooks.json to add hooks.",
}];

const NOTES: &[&str] = &[
    "Reads from spec/fspec-hooks.json",
    "Shows event names and hook names only (not full configuration)",
    "Use validate-hooks to check if hook scripts exist",
    "Hooks are organized by event (pre-/post- command pattern)",
];

const RELATED: &[&str] = &["validate-hooks", "add-hook", "remove-hook"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "list-hooks",
    description: "List all configured lifecycle hooks",
    usage: Some("fspec list-hooks"),
    arguments: &[],
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command to see what hooks are configured for your project, including their event names and hook names. Useful for understanding the current automation setup and debugging hook execution.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: Some(
        "fspec list-hooks → Review configured hooks → fspec validate-hooks → fspec add-hook (if needed)",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
