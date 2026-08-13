//! `clear-virtual-hooks` help configuration — Rust port of
//! `src/commands/clear-virtual-hooks-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js clear-virtual-hooks --help`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError, CommonPattern,
    CommonPatternEntry,
};

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec clear-virtual-hooks AUTH-001",
        description: Some("Clear all virtual hooks from AUTH-001"),
        output: Some("✓ Cleared 3 virtual hook(s) from AUTH-001"),
    },
    CommandExample {
        command: "fspec clear-virtual-hooks BUG-042",
        description: Some("Clear hooks from work unit with no hooks"),
        output: Some("✓ Cleared 0 virtual hook(s) from BUG-042"),
    },
];

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "workUnitId",
    description: "Work unit ID to clear hooks from (e.g., AUTH-001)",
    required: true,
}];

const OPTIONS: &[CommandOption] = &[];

const COMMON_ERRORS: &[CommonError] = &[CommonError {
    error: "Work unit 'AUTH-001' does not exist",
    fix: "Check work unit ID spelling. List all work units: fspec list-work-units",
}];

const RELATED: &[&str] = &[
    "add-virtual-hook",
    "list-virtual-hooks",
    "remove-virtual-hook",
    "update-work-unit-status",
];

const NOTES: &[&str] = &[
    "Clears ALL virtual hooks - operation cannot be undone",
    "Automatically cleans up all script files in spec/hooks/.virtual/",
    "Sets virtualHooks array to empty [] (not undefined)",
    "Operation is safe even if work unit has no hooks (clears 0 hooks)",
    "AI prompts for this decision when work unit transitions to \"done\" status",
    "Consider using remove-virtual-hook to selectively remove specific hooks",
];

const PREREQUISITES: &[&str] = &["Work unit must exist"];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Cleanup After Story Completion",
        example: "fspec update-work-unit-status AUTH-001 done\n# AI asks: \"Keep or remove hooks?\"\n# User chooses: \"remove\"\nfspec clear-virtual-hooks AUTH-001",
        description: "Standard workflow when completing a work unit and removing temporary quality checks.",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Reset Work Unit Configuration",
        example: "fspec clear-virtual-hooks AUTH-001\n# Start fresh with new hooks:\nfspec add-virtual-hook AUTH-001 post-implementing \"<quality-check-commands>\" --blocking",
        description: "Clear all existing hooks to start with a clean slate and add new configuration.",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Bulk Cleanup After Release",
        example: "fspec list-work-units --status=done\n# For each done work unit:\nfspec clear-virtual-hooks AUTH-001\nfspec clear-virtual-hooks AUTH-002",
        description: "Clean up virtual hooks from multiple completed work units after a release.",
    }),
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "clear-virtual-hooks",
    description: "Clear all virtual hooks from a work unit",
    usage: Some("fspec clear-virtual-hooks <workUnitId>"),
    arguments: ARGUMENTS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command when a work unit reaches \"done\" status and you want to remove all virtual hooks. Clears the entire virtualHooks array and cleans up all generated script files in spec/hooks/.virtual/. Useful for cleanup after story completion or when resetting work unit configuration.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(
        "Work unit reaches done → AI asks \"Keep or remove hooks?\" → User chooses remove → fspec clear-virtual-hooks",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
