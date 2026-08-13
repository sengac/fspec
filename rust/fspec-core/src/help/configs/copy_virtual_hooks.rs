//! `copy-virtual-hooks` help configuration — Rust port of
//! `src/commands/copy-virtual-hooks-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js copy-virtual-hooks --help`.

use super::super::{
    CommandExample, CommandHelpConfig, CommandOption, CommonError, CommonPattern,
    CommonPatternEntry,
};

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002",
        description: Some("Copy all virtual hooks from AUTH-001 to AUTH-002"),
        output: Some("✓ Copied 3 virtual hook(s) from AUTH-001 to AUTH-002"),
    },
    CommandExample {
        command: "fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002 --hook-name eslint",
        description: Some("Copy only the eslint hook from AUTH-001 to AUTH-002"),
        output: Some("✓ Copied 1 virtual hook(s) from AUTH-001 to AUTH-002"),
    },
    CommandExample {
        command: "fspec copy-virtual-hooks --from FEAT-100 --to FEAT-101",
        description: Some("Copy hooks to a related feature work unit"),
        output: Some("✓ Copied 2 virtual hook(s) from FEAT-100 to FEAT-101"),
    },
];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--from <workUnitId>",
        description: "Source work unit ID to copy hooks from (e.g., AUTH-001). Required.",
        default_value: None,
    },
    CommandOption {
        flag: "--to <workUnitId>",
        description: "Target work unit ID to copy hooks to (e.g., AUTH-002). Required.",
        default_value: None,
    },
    CommandOption {
        flag: "--hook-name <name>",
        description:
            "Copy only a specific hook by name (e.g., eslint). Optional - omit to copy all hooks.",
        default_value: None,
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "--from option is required",
        fix: "Specify source work unit: --from AUTH-001",
    },
    CommonError {
        error: "--to option is required",
        fix: "Specify target work unit: --to AUTH-002",
    },
    CommonError {
        error: "Source work unit 'AUTH-999' does not exist",
        fix: "Check source work unit ID. List work units: fspec list-work-units",
    },
    CommonError {
        error: "Target work unit 'AUTH-888' does not exist",
        fix: "Create target work unit first: fspec create-story AUTH \"Title\" (or create-bug/create-task)",
    },
    CommonError {
        error: "No virtual hooks configured for source work unit AUTH-001",
        fix: "Source must have hooks. Add hooks: fspec add-virtual-hook AUTH-001 ...",
    },
    CommonError {
        error: "Hook 'eslin' not found in AUTH-001",
        fix: "Check hook name spelling. List hooks: fspec list-virtual-hooks AUTH-001",
    },
];

const RELATED: &[&str] = &[
    "add-virtual-hook",
    "list-virtual-hooks",
    "remove-virtual-hook",
];

const NOTES: &[&str] = &[
    "Copies hook configuration, not generated script files",
    "Target work unit can have existing hooks - copied hooks are appended",
    "Copied hooks maintain all settings: event, command, blocking, git context",
    "Operation creates deep copy - modifying source hooks does not affect copies",
    "Use --hook-name to copy selectively, omit to copy all hooks",
    "Script files for git context hooks are NOT copied (regenerated on execution)",
];

const PREREQUISITES: &[&str] = &[
    "Source work unit must exist",
    "Target work unit must exist",
    "Source work unit must have virtual hooks configured",
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Copy All Hooks to Related Stories",
        example: "fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002\nfspec copy-virtual-hooks --from AUTH-001 --to AUTH-003",
        description: "Apply same quality checks to all authentication-related work units.",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Copy Specific Hook to Multiple Work Units",
        example: "fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002 --hook-name eslint\nfspec copy-virtual-hooks --from AUTH-001 --to AUTH-003 --hook-name eslint",
        description: "Apply only linting check to multiple work units, skip other hooks.",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Create Template and Copy to New Work Units",
        example: "# Set up template:\nfspec add-virtual-hook TEMPLATE-001 post-implementing \"<quality-check-commands>\" --blocking\nfspec add-virtual-hook TEMPLATE-001 pre-validating \"<quality-check-commands>\" --blocking\n\n# Copy to actual work units:\nfspec copy-virtual-hooks --from TEMPLATE-001 --to AUTH-010\nfspec copy-virtual-hooks --from TEMPLATE-001 --to BUG-020",
        description: "Create a template work unit with standard hooks, then copy to new work units.",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Verify After Copy",
        example: "fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002\nfspec list-virtual-hooks AUTH-002",
        description: "Copy hooks and immediately verify they were copied correctly.",
    }),
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "copy-virtual-hooks",
    description: "Copy virtual hooks from one work unit to another",
    usage: Some("fspec copy-virtual-hooks --from <sourceId> --to <targetId> [--hook-name <name>]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command to copy virtual hooks between work units. Useful when multiple related work units need the same quality checks (e.g., all authentication stories need eslint + prettier). Copies hook configuration including event, command, blocking status, and git context settings.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(
        "Create related work units → Configure hooks on first → Copy to others → Verify with list-virtual-hooks",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
