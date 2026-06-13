//! `remove-virtual-hook` help configuration — Rust port of
//! `src/commands/remove-virtual-hook-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js remove-virtual-hook --help`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommonError, CommonPattern,
    CommonPatternEntry,
};

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec remove-virtual-hook AUTH-001 eslint",
        description: Some("Remove the eslint hook from AUTH-001"),
        output: Some("✓ Removed virtual hook 'eslint' from AUTH-001\n  Remaining virtual hooks: 1"),
    },
    CommandExample {
        command: "fspec remove-virtual-hook BUG-042 typecheck",
        description: Some("Remove type check hook from BUG-042"),
        output: Some(
            "✓ Removed virtual hook 'typecheck' from BUG-042\n  Remaining virtual hooks: 0",
        ),
    },
];

const ARGUMENTS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "Work unit ID to remove hook from (e.g., AUTH-001)",
        required: true,
    },
    CommandArgument {
        name: "hookName",
        description: "Name of the hook to remove (e.g., eslint, prettier, typecheck)",
        required: true,
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Work unit 'AUTH-001' does not exist",
        fix: "Check work unit ID spelling. List all work units: fspec list-work-units",
    },
    CommonError {
        error: "No virtual hooks configured for AUTH-001",
        fix: "Work unit has no hooks to remove. List hooks: fspec list-virtual-hooks AUTH-001",
    },
    CommonError {
        error: "Virtual hook 'eslin' not found in AUTH-001",
        fix: "Check hook name spelling. List hooks: fspec list-virtual-hooks AUTH-001 (correct name might be \"eslint\")",
    },
];

const RELATED: &[&str] = &[
    "add-virtual-hook",
    "list-virtual-hooks",
    "clear-virtual-hooks",
];

const NOTES: &[&str] = &[
    "Hook name is auto-generated from command (e.g., \"<quality-check-commands>\" → based on actual tool)",
    "To find hook names, use: fspec list-virtual-hooks <workUnitId>",
    "Automatically cleans up script files in spec/hooks/.virtual/",
    "If multiple hooks have same name at different events, only first is removed",
    "To remove ALL hooks, use: fspec clear-virtual-hooks",
    "Operation is permanent - hook configuration cannot be recovered",
];

const PREREQUISITES: &[&str] = &[
    "Work unit must exist",
    "Work unit must have virtual hooks configured",
    "Hook name must match exactly (case-sensitive)",
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Remove Broken Hook",
        example:
            "fspec remove-virtual-hook AUTH-001 eslin\nfspec add-virtual-hook AUTH-001 post-implementing \"eslint src/\" --blocking",
        description: "Remove hook with typo in command name, then add corrected version.",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Replace Hook Configuration",
        example:
            "fspec remove-virtual-hook AUTH-001 eslint\nfspec add-virtual-hook AUTH-001 post-implementing \"eslint src/\" --git-context --blocking",
        description:
            "Remove existing hook and add new one with different configuration (e.g., add git context).",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Remove After Workflow Transition",
        example:
            "fspec update-work-unit-status AUTH-001 done\n# Hook failed, no longer needed:\nfspec remove-virtual-hook AUTH-001 experimental-check",
        description: "Remove experimental or one-time hooks after work unit completion.",
    }),
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "remove-virtual-hook",
    description: "Remove a specific virtual hook from a work unit",
    usage: Some("fspec remove-virtual-hook <workUnitId> <hookName>"),
    arguments: ARGUMENTS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command to remove a specific virtual hook from a work unit. Removes the hook configuration from spec/work-units.json and cleans up any generated script files in spec/hooks/.virtual/. Useful for removing broken hooks, hooks that are no longer needed, or correcting mistakes.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(
        "fspec list-virtual-hooks → Identify hook to remove → fspec remove-virtual-hook → Verify removal",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
