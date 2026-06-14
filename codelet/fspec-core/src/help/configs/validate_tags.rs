//! `validate-tags` help configuration — Rust port of
//! `src/commands/validate-tags-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js validate-tags --help`.
//! Captured fixture: `codelet/fspec/tests/fixtures/help/validate-tags.txt`.

use super::super::{CommandExample, CommandHelpConfig, CommonError};

const EXAMPLE_OUTPUT: &str = "✓ All tags in spec/features/login.feature are registered\n✓ All tags in spec/features/signup.feature are registered\n\n✓ 2 files passed";

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec validate-tags",
    description: Some("Validate all tags"),
    output: Some(EXAMPLE_OUTPUT),
}];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Tag @my-tag is not registered",
        fix: "Register the tag: fspec register-tag @my-tag \"Category\" \"Description\"",
    },
    CommonError {
        error: "Work unit ID tag @AUTH-001 must be at feature level, not scenario level",
        fix: "Move the work unit ID tag from scenario-level to feature-level tags. Use coverage files (*.feature.coverage) for fine-grained scenario traceability instead of scenario-level work unit tags.",
    },
];

const RELATED: &[&str] = &["register-tag", "list-tags", "check"];

const NOTES: &[&str] = &[
    "Part of fspec check command",
    "Exit code 0 = all valid, non-zero = unregistered tags found or placement violations",
    "Work unit ID tags (e.g., AUTH-001, COV-005) must be at feature level only",
    "Use coverage files for linking scenarios to implementation (two-tier linking system)",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "validate-tags",
    description: "Validate that all tags in feature files are registered in spec/tags.json and enforce tag placement rules",
    usage: Some("fspec validate-tags"),
    arguments: &[],
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use to ensure all tags used in feature files are properly registered and correctly placed. Validates that work unit ID tags appear only at feature level (not scenario level). Run before committing to catch unregistered tags and placement violations.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
