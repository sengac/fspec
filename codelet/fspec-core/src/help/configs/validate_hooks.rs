//! `validate-hooks` help configuration — Rust port of
//! `src/commands/validate-hooks-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js validate-hooks --help`.
//! Captured fixture: `codelet/fspec/tests/fixtures/help/validate-hooks.txt`.

use super::super::{CommandExample, CommandHelpConfig, CommonError};

const EXAMPLE_MISSING_OUTPUT: &str = "✗ Hook validation failed\n\nHook command not found: spec/hooks/validate-feature.sh\nHook command not found: spec/hooks/lint.sh\n\nFix these issues before using hooks.";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec validate-hooks",
        description: Some("When all hooks are valid"),
        output: Some("✓ All hooks are valid"),
    },
    CommandExample {
        command: "fspec validate-hooks",
        description: Some("When hook scripts are missing"),
        output: Some(EXAMPLE_MISSING_OUTPUT),
    },
    CommandExample {
        command: "fspec validate-hooks",
        description: Some("When no hooks are configured"),
        output: Some("No hooks configured (nothing to validate)"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Hook command not found: spec/hooks/my-hook.sh",
        fix: "Create the hook script at the specified path or update the hook configuration to point to the correct path. Remember: paths must be relative to project root.",
    },
    CommonError {
        error: "Failed to load hook configuration",
        fix: "Check that spec/fspec-hooks.json exists and contains valid JSON. Run fspec add-hook to create the file if missing.",
    },
];

const RELATED: &[&str] = &["list-hooks", "add-hook", "remove-hook"];

const NOTES: &[&str] = &[
    "Validates that all hook scripts exist at specified paths",
    "Does NOT execute hooks (only checks existence)",
    "Does NOT validate hook script syntax or permissions",
    "Exit code 0 = valid, non-zero = errors found",
    "Run after adding or modifying hooks",
];

const PREREQUISITES: &[&str] = &[
    "Hook configuration file exists (spec/fspec-hooks.json)",
    "At least one hook is configured",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "validate-hooks",
    description: "Validate hook configuration and verify that all hook scripts exist",
    usage: Some("fspec validate-hooks"),
    arguments: &[],
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command to verify that your hook configuration is valid and all hook scripts exist at the specified paths. Essential before relying on hooks for workflow automation.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    typical_workflow: Some(
        "Create hook script → chmod +x script → fspec add-hook → fspec validate-hooks → Test hook execution",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
