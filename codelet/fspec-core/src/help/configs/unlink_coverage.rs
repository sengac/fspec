//! `unlink-coverage` help configuration — Rust port of
//! `src/commands/unlink-coverage-help.ts` (RPC-311).
//!
//! Byte-for-byte parity with `node dist/index.js unlink-coverage --help`
//! piped to non-TTY. Reference fixture:
//! `codelet/fspec/tests/fixtures/help/unlink-coverage.txt`.
//!
//! ## Quirk: trailing literal `undefined` in COMMON PATTERNS
//! The TS commonPatterns entries carry `{pattern, example}` but the formatter
//! also reads `description`, which is absent — so each pattern's third line
//! renders as the literal text `undefined`. Reproduced verbatim.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError,
    CommonPattern, CommonPatternEntry,
};

const PATTERN_1_EXAMPLE: &str = r#"# Remove all mappings to start fresh
fspec unlink-coverage user-auth --scenario "Login" --all

# Re-link with correct paths
fspec link-coverage user-auth --scenario "Login" --test-file <correct-path> --test-lines <range>"#;
const PATTERN_2_EXAMPLE: &str = r#"# Implementation was refactored, need to re-link
fspec unlink-coverage user-auth --scenario "Login" --test-file src/__tests__/auth.test.ts --impl-file src/auth/old-login.ts

# Link new implementation
fspec link-coverage user-auth --scenario "Login" --test-file src/__tests__/auth.test.ts --impl-file src/auth/new-login.ts --impl-lines 5-15"#;
const PATTERN_3_EXAMPLE: &str = r#"# Test file was deleted
fspec unlink-coverage user-auth --scenario "Login" --test-file src/__tests__/old-auth.test.ts

# Or use audit-coverage --fix
fspec audit-coverage user-auth --fix"#;

const ARGUMENTS: &[CommandArgument] = &[
    CommandArgument {
        name: "feature-name",
        description: "Feature file name (without path or extension), e.g., \"user-authentication\"",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--scenario <name>",
        description: "Scenario name to unlink (required)",
        default_value: None,
    },
    CommandOption {
        flag: "--test-file <path>",
        description: "Test file path to remove. If specified with --impl-file, removes only impl mapping.",
        default_value: None,
    },
    CommandOption {
        flag: "--impl-file <path>",
        description: "Implementation file path to remove. Removes implementation mapping from test.",
        default_value: None,
    },
    CommandOption {
        flag: "--all",
        description: "Remove all mappings for the scenario (reset to uncovered)",
        default_value: None,
    },
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Reset Scenario Coverage",
        example: PATTERN_1_EXAMPLE,
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Remove Implementation Only",
        example: PATTERN_2_EXAMPLE,
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Clean Up After Deletion",
        example: PATTERN_3_EXAMPLE,
        description: "undefined",
    }),
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec unlink-coverage user-authentication --scenario \"Login with valid credentials\" --all",
        description: Some("Remove all coverage mappings for a scenario"),
        output: Some("✓ Removed all coverage mappings for scenario \"Login with valid credentials\""),
    },
    CommandExample {
        command: "fspec unlink-coverage user-authentication --scenario \"Login with valid credentials\" --test-file src/__tests__/auth.test.ts",
        description: Some("Remove entire test mapping (including implementation links)"),
        output: Some("✓ Removed test mapping src/__tests__/auth.test.ts from scenario \"Login with valid credentials\""),
    },
    CommandExample {
        command: "fspec unlink-coverage user-authentication --scenario \"Login with valid credentials\" --test-file src/__tests__/auth.test.ts --impl-file src/auth/login.ts",
        description: Some("Remove only implementation mapping, keep test mapping"),
        output: Some("✓ Removed implementation src/auth/login.ts from test mapping src/__tests__/auth.test.ts"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Error: Scenario \"Login with valid credentials\" not found in feature",
        fix: "Check scenario name matches exactly. Run: fspec show-feature user-authentication",
    },
    CommonError {
        error: "Error: Test file src/__tests__/auth.test.ts not found in scenario mappings",
        fix: "This test is not linked to the scenario. Run: fspec show-coverage user-authentication",
    },
    CommonError {
        error: "Error: Must specify --scenario with --test-file or --impl-file",
        fix: "Add --scenario flag to specify which scenario to unlink from.",
    },
];

const RELATED: &[&str] = &[
    "link-coverage",
    "show-coverage",
    "audit-coverage",
];

const NOTES: &[&str] = &[
    "Use --all to completely reset scenario coverage (makes it uncovered)",
    "Unlinking test file also removes all its implementation mappings",
    "Unlinking only implementation keeps test mapping intact",
    "Consider using audit-coverage --fix instead for batch cleanup",
    "Does not modify feature files or test/implementation files, only .coverage files",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "unlink-coverage",
    description: "Remove test or implementation links from scenario coverage mappings",
    usage: Some("fspec unlink-coverage <feature-name> [options]"),
    arguments: ARGUMENTS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some("Use this command when tests or implementation are deleted, refactored, or when coverage mappings are incorrect and need to be reset. Useful before re-linking with correct file paths."),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some("1. Identify incorrect mapping with fspec show-coverage → 2. fspec unlink-coverage → 3. Re-link with correct paths using fspec link-coverage"),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
