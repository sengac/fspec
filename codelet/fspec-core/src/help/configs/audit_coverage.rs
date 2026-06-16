//! `audit-coverage` help configuration — Rust port of
//! `src/commands/audit-coverage-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js audit-coverage --help`
//! piped to non-TTY (captured fixture:
//! `codelet/fspec/tests/fixtures/help/audit-coverage.txt`).
//!
//! TS quirks preserved for parity:
//!   1. `common_patterns` in TS specify `pattern` + `example` but NO
//!      `description`; the help formatter renders `${p.description}` which is
//!      `undefined`, producing a literal `undefined` line after each
//!      `Example:` block. We model each as a `Structured` entry whose
//!      `description` is the literal string `"undefined"`.
//!   2. `common_errors` in TS specify `fix:` directly (NOT `solution:`), so
//!      these render the real fix text (no `undefined` quirk here, unlike
//!      list-attachments).
//!   3. `related_commands` are bare (no `fspec ` prefix in source), so the
//!      formatter's single prefix produces `fspec show-coverage` etc. — the
//!      fixture shows the single-prefixed form.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError, CommonPattern,
    CommonPatternEntry,
};

const ARGS: &[CommandArgument] = &[CommandArgument {
    name: "feature-name",
    description: "Feature file name (without path or extension), e.g., \"user-authentication\"",
    required: true,
}];

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "--fix",
    description: "Automatically remove broken mappings from coverage file",
    default_value: None,
}];

const EXAMPLE_1_OUTPUT: &str = "Auditing: user-authentication.feature\n\n✓ Login with valid credentials\n  ✓ Test file exists: src/__tests__/auth.test.ts\n  ✓ Implementation file exists: src/auth/login.ts\n\n✗ Login with invalid credentials\n  ✗ Test file missing: src/__tests__/auth-error.test.ts\n  ✓ Implementation file exists: src/auth/login.ts\n\nAudit Summary:\n- 2 scenarios checked\n- 1 fully valid\n- 1 has broken links\n\nRun with --fix to remove broken mappings";

const EXAMPLE_2_OUTPUT: &str = "Auditing: user-authentication.feature\n\n✗ Login with invalid credentials\n  ✗ Test file missing: src/__tests__/auth-error.test.ts\n  Removed broken test mapping\n\n✓ Fixed 1 broken mapping";

const EXAMPLE_3_OUTPUT: &str = "Auditing: dashboard.feature\n\n✓ View dashboard metrics\n  ✓ Test file exists: src/__tests__/dashboard.test.ts\n  ✓ Implementation file exists: src/dashboard/metrics.ts\n\nAll mappings valid! ✅";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec audit-coverage user-authentication",
        description: Some("Audit coverage for specific feature"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec audit-coverage user-authentication --fix",
        description: Some("Audit and automatically fix broken links"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
    CommandExample {
        command: "fspec audit-coverage dashboard",
        description: Some("Audit coverage after moving files"),
        output: Some(EXAMPLE_3_OUTPUT),
    },
];

const PATTERN_1_EXAMPLE: &str = "# Moved test files to new directory structure\nmv src/__tests__/auth/* src/auth/__tests__/\n\n# Audit all features to find broken links\nfspec list-features --format=json | jq -r '.[].name' | while read feature; do\n  fspec audit-coverage \"$feature\"\ndone";

const PATTERN_2_EXAMPLE: &str = "# After deleting obsolete test files\nfspec audit-coverage user-authentication\n# Shows broken links\n\n# Auto-fix (removes broken mappings)\nfspec audit-coverage user-authentication --fix";

const PATTERN_3_EXAMPLE: &str =
    "# In CI pipeline, fail if any coverage links are broken\nfspec audit-coverage user-authentication || exit 1";

// TS quirk #1: source patterns have no `description`; formatter renders
// `undefined`. Model each as a Structured entry with the literal "undefined".
const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "After Refactoring",
        example: PATTERN_1_EXAMPLE,
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Fix Broken Links Automatically",
        example: PATTERN_2_EXAMPLE,
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "CI/CD Validation",
        example: PATTERN_3_EXAMPLE,
        description: "undefined",
    }),
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Error: Feature file user-authentication.feature not found",
        fix: "Check feature name matches file name. Run: fspec list-features",
    },
    CommonError {
        error: "Error: Coverage file user-authentication.feature.coverage not found",
        fix: "Coverage file should auto-create with fspec create-feature.",
    },
];

const RELATED: &[&str] = &["show-coverage", "link-coverage", "unlink-coverage"];

const NOTES: &[&str] = &[
    "Does NOT validate that line numbers are correct, only that files exist",
    "Use --fix to automatically remove broken mappings (backup first!)",
    "Run after moving or renaming files to catch stale references",
    "Essential part of refactoring workflow to maintain coverage integrity",
    "Does not modify feature files, only .coverage files",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "audit-coverage",
    description:
        "Verify that test files and implementation files referenced in coverage mappings actually exist",
    usage: Some("fspec audit-coverage <feature-name> [options]"),
    arguments: ARGS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command after refactoring to verify file paths are still correct, or periodically to detect broken coverage links. Essential before deploying or after moving files. Catches issues where tests/implementation were deleted but coverage file was not updated.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(
        "1. Refactor/move files → 2. fspec audit-coverage → 3. Fix broken paths manually OR use --fix → 4. Verify with fspec show-coverage",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
