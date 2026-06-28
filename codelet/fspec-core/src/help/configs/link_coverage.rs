//! `link-coverage` help configuration — Rust port of
//! `src/commands/link-coverage-help.ts` (RPC-240).
//!
//! Byte-for-byte parity with `node dist/index.js link-coverage --help` piped to
//! non-TTY. Reference fixture: `codelet/fspec/tests/fixtures/help/link-coverage.txt`.
//!
//! ## Quirk: trailing literal `undefined` in COMMON PATTERNS
//! The TS commonPatterns entries carry `{pattern, example}` but the formatter
//! also reads `description`, which is absent — so each pattern's third line
//! renders as the literal text `undefined`. Reproduced verbatim.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError, CommonPattern,
    CommonPatternEntry,
};

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "feature-name",
    description: "Feature file name (without path or extension), e.g., \"user-authentication\"",
    required: true,
}];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--scenario <name>",
        description: "Scenario name to link (required)",
        default_value: None,
    },
    CommandOption {
        flag: "--test-file <path>",
        description: "Test file path (required for linking test, e.g., \"src/__tests__/auth.test.ts\")",
        default_value: None,
    },
    CommandOption {
        flag: "--test-lines <range>",
        description: "Test line range (e.g., \"45-62\" or \"45,46,47\"). Used when linking test file.",
        default_value: None,
    },
    CommandOption {
        flag: "--impl-file <path>",
        description: "Implementation file path (e.g., \"src/auth/login.ts\"). Used to link implementation.",
        default_value: None,
    },
    CommandOption {
        flag: "--impl-lines <lines>",
        description: "Implementation line numbers (e.g., \"10,11,12,23,24\"). Used with --impl-file.",
        default_value: None,
    },
    CommandOption {
        flag: "--skip-validation",
        description: "Skip file path validation (useful for skeleton tests in reverse ACDD)",
        default_value: None,
    },
    CommandOption {
        flag: "--skip-step-validation",
        description: "Skip step comment validation (ONLY allowed for task work units - story and bug work units require MANDATORY step validation)",
        default_value: None,
    },
];

const PATTERN_1_EXAMPLE: &str = "# After writing tests\n<test-command>  # Tests fail (red)\nfspec link-coverage user-auth --scenario \"Login\" --test-file src/__tests__/auth.test.ts --test-lines 45-62\n\n# After implementing\n<test-command>  # Tests pass (green)\nfspec link-coverage user-auth --scenario \"Login\" --test-file src/__tests__/auth.test.ts --impl-file src/auth/login.ts --impl-lines 10-24";
const PATTERN_2_EXAMPLE: &str = "# Link skeleton test (not implemented yet)\nfspec link-coverage user-login --scenario \"Login\" --test-file src/__tests__/auth.test.ts --test-lines 13-27 --skip-validation\n\n# Link existing implementation\nfspec link-coverage user-login --scenario \"Login\" --test-file src/__tests__/auth.test.ts --impl-file src/routes/auth.ts --impl-lines 45-67 --skip-validation";

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Forward ACDD Workflow",
        example: PATTERN_1_EXAMPLE,
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Reverse ACDD Workflow",
        example: PATTERN_2_EXAMPLE,
        description: "undefined",
    }),
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec link-coverage user-authentication --scenario \"Login with valid credentials\" --test-file src/__tests__/auth.test.ts --test-lines 45-62",
        description: Some("Link test file to scenario (after writing tests)"),
        output: Some("✓ Linked test to scenario \"Login with valid credentials\"\n  Test: src/__tests__/auth.test.ts:45-62"),
    },
    CommandExample {
        command: "fspec link-coverage user-authentication --scenario \"Login with valid credentials\" --test-file src/__tests__/auth.test.ts --impl-file src/auth/login.ts --impl-lines 10-24",
        description: Some("Link implementation to existing test mapping (after implementing)"),
        output: Some("✓ Linked implementation to scenario \"Login with valid credentials\"\n  Test: src/__tests__/auth.test.ts:45-62\n  Implementation: src/auth/login.ts:10,11,12,23,24"),
    },
    CommandExample {
        command: "fspec link-coverage user-authentication --scenario \"Login with valid credentials\" --test-file src/__tests__/auth.test.ts --test-lines 45-62 --impl-file src/auth/login.ts --impl-lines 10-24",
        description: Some("Link both test and implementation at once"),
        output: Some("✓ Linked test and implementation to scenario \"Login with valid credentials\"\n  Test: src/__tests__/auth.test.ts:45-62\n  Implementation: src/auth/login.ts:10,11,12,23,24"),
    },
    CommandExample {
        command: "fspec link-coverage user-login --scenario \"Login with valid credentials\" --test-file src/__tests__/auth-login.test.ts --test-lines 13-27 --skip-validation",
        description: Some("Link skeleton test in reverse ACDD (use --skip-validation for unimplemented tests)"),
        output: Some("✓ Linked skeleton test to scenario \"Login with valid credentials\" (validation skipped)\n  Test: src/__tests__/auth-login.test.ts:13-27"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Error: Scenario \"Login with valid credentials\" not found in feature",
        fix: "Check scenario name matches exactly. Run: fspec show-feature user-authentication",
    },
    CommonError {
        error: "Error: Test file src/__tests__/auth.test.ts does not exist",
        fix: "Verify file path is correct. Use --skip-validation for reverse ACDD skeleton tests.",
    },
    CommonError {
        error: "Error: Cannot link implementation without existing test mapping",
        fix: "Link test file first, then link implementation to that test mapping.",
    },
    CommonError {
        error: "Step validation failed: Missing step comment \"When I click the login button\"",
        fix: "Add step comments to test file: // @step When I click the login button\nNote: --skip-step-validation is ONLY allowed for task work units",
    },
];

const RELATED: &[&str] = &[
    "show-coverage",
    "audit-coverage",
    "unlink-coverage",
    "create-feature",
];

const NOTES: &[&str] = &[
    "Coverage files (.feature.coverage) are auto-created by fspec create-feature",
    "ALWAYS link coverage immediately after writing tests or code (do not batch)",
    "Use --skip-validation in reverse ACDD for skeleton tests and forward planning",
    "Coverage tracking is CRITICAL for reverse ACDD to track mapping progress",
    "Line ranges: \"45-62\" for range, \"10,11,12,23,24\" for specific lines",
    "Step validation: Test files must include step comments (// @step Given...) matching feature file steps",
    "Step comments support both @step prefix (recommended) and plain format (backward compatible)",
    "Parameterized steps ({int}, {string}) match via hybrid similarity algorithm",
    "Step validation fails with helpful system-reminder showing exact text to add to test file",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "link-coverage",
    description: "Link Gherkin scenarios to test files and implementation code for full traceability",
    usage: Some("fspec link-coverage <feature-name> [options]"),
    arguments: ARGUMENTS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some("Use this command IMMEDIATELY after writing tests or implementation code to maintain scenario-to-test-to-code traceability. CRITICAL for reverse ACDD to track what has been mapped and what remains. Essential for refactoring safety and gap detection."),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some("1. Write tests (red phase) → 2. fspec link-coverage --test-file → 3. Implement code (green phase) → 4. fspec link-coverage --impl-file → 5. Verify with fspec show-coverage"),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
