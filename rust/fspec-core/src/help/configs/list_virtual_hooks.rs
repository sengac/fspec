//! `list-virtual-hooks` help configuration — Rust port of
//! `src/commands/list-virtual-hooks-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js list-virtual-hooks --help`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError, CommonPattern,
    CommonPatternEntry,
};

const EXAMPLE_1_OUTPUT: &str = "Virtual Hooks for AUTH-001:\n\n  post-implementing:\n    • eslint [blocking]\n      <quality-check-commands>\n    • prettier [non-blocking]\n      prettier --check .\n\n  pre-validating:\n    • typecheck [blocking]\n      <quality-check-commands>";

const EXAMPLE_3_OUTPUT: &str = "Virtual Hooks for FEAT-123:\n\n  post-implementing:\n    • eslint [blocking] [git-context]\n      spec/hooks/.virtual/FEAT-123-eslint.sh";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec list-virtual-hooks AUTH-001",
        description: Some("List all virtual hooks for AUTH-001"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec list-virtual-hooks BUG-042",
        description: Some("List hooks for a work unit with no virtual hooks"),
        output: Some("No virtual hooks configured for BUG-042"),
    },
    CommandExample {
        command: "fspec list-virtual-hooks FEAT-123",
        description: Some("List hooks including git context hooks"),
        output: Some(EXAMPLE_3_OUTPUT),
    },
];

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "workUnitId",
    description: "Work unit ID to list hooks for (e.g., AUTH-001)",
    required: true,
}];

const OPTIONS: &[CommandOption] = &[];

const COMMON_ERRORS: &[CommonError] = &[CommonError {
    error: "Work unit 'AUTH-001' does not exist",
    fix: "Check work unit ID spelling or create work unit first: fspec create-story AUTH \"Title\" (or create-bug/create-task)",
}];

const RELATED: &[&str] = &[
    "add-virtual-hook",
    "remove-virtual-hook",
    "clear-virtual-hooks",
    "show-work-unit",
];

const NOTES: &[&str] = &[
    "Hooks are displayed grouped by event for clarity",
    "[blocking] badge indicates hook failure prevents workflow transition",
    "[git-context] badge indicates hook uses git staged/unstaged files",
    "Hooks execute in the order they were added (top to bottom)",
    "Virtual hooks also appear in \"fspec show-work-unit\" output",
    "Empty output means no virtual hooks configured for the work unit",
];

const PREREQUISITES: &[&str] = &["Work unit must exist"];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Review Quality Gates Before Implementing",
        example: "fspec list-virtual-hooks AUTH-001\n# Review hooks, then:\nfspec update-work-unit-status AUTH-001 implementing",
        description: "Check what quality checks will run before starting implementation.",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Debug Hook Execution",
        example: "fspec list-virtual-hooks AUTH-001\n# If hook fails, review command and fix",
        description: "When a hook fails, list hooks to see the exact command being executed.",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Compare Hooks Across Work Units",
        example: "fspec list-virtual-hooks AUTH-001\nfspec list-virtual-hooks AUTH-002",
        description: "Compare quality checks between related work units to ensure consistency.",
    }),
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "list-virtual-hooks",
    description: "List all virtual hooks configured for a work unit",
    usage: Some("fspec list-virtual-hooks <workUnitId>"),
    arguments: ARGUMENTS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command to view all virtual hooks attached to a specific work unit. Shows hooks grouped by event with their configuration (blocking, git context, command). Useful for reviewing quality checks before workflow transitions or debugging hook execution.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(
        "Add virtual hooks → fspec list-virtual-hooks → Review configuration → Update hooks if needed → Execute workflow transition",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
