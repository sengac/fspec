//! `validate-spec-alignment` help configuration — Rust port of
//! `src/commands/validate-spec-alignment-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js validate-spec-alignment
//! --help`. Captured fixture:
//! `codelet/fspec/tests/fixtures/help/validate-spec-alignment.txt`.
//!
//! NOTE (Framing A): the help doc still advertises the BROKEN
//! `[feature-files...]` positional + `--fix` shape because the help text is
//! the captured-fixture canon. The actual clap surface exposes a required
//! `<workUnitId>` (mirroring the real exported contract). This config
//! reproduces the TS help byte-for-byte regardless.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError, CommonPattern,
    CommonPatternEntry,
};

const ARGS: &[CommandArgument] = &[CommandArgument {
    name: "feature-files",
    description: "Feature files to validate (default: all in spec/features). Can be file paths or feature names.",
    required: false,
}];

const OPTS: &[CommandOption] = &[CommandOption {
    flag: "--fix",
    description: "Automatically fix alignment issues (e.g., create missing coverage mappings)",
    default_value: None,
}];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec validate-spec-alignment",
        description: Some("Validate alignment for all feature files"),
        output: Some(
            "✓ All specs are aligned with tests and implementation\n\nAlignment summary:\n  Features checked: 12\n  Scenarios checked: 45\n  Scenarios with tests: 45\n  Test coverage: 100%",
        ),
    },
    CommandExample {
        command: "fspec validate-spec-alignment spec/features/auth/login.feature",
        description: Some("Validate specific feature file"),
        output: Some("✓ All specs are aligned with tests and implementation"),
    },
    CommandExample {
        command: "fspec validate-spec-alignment spec/features/auth/*.feature --fix",
        description: Some("Validate and auto-fix alignment issues"),
        output: Some(
            "✓ Fixed 2 alignment issues\n✓ All specs are aligned with tests and implementation",
        ),
    },
    CommandExample {
        command: "fspec validate-spec-alignment",
        description: Some("Example with alignment issues found"),
        output: Some(
            "✗ Found 3 alignment issues\n  - Scenario \"Valid user login\" has no test mapping\n  - Scenario \"Invalid credentials\" test has no implementation\n  - Feature file \"checkout.feature\" has no @WORK-UNIT-ID tags",
        ),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Found 3 alignment issues",
        fix: "Review listed issues and either:\n1. Add test mappings: fspec link-coverage <feature> --scenario \"<name>\" --test-file <file> --test-lines <range>\n2. Add implementation: fspec link-coverage <feature> --scenario \"<name>\" --test-file <file> --impl-file <file> --impl-lines <lines>\n3. Tag scenarios: fspec add-tag-to-scenario <feature> \"<scenario>\" @WORK-UNIT-ID",
    },
    CommonError {
        error: "Feature file not found: spec/features/nonexistent.feature",
        fix: "Check feature file path or use: fspec list-features",
    },
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Validate before moving to done",
        example: "# Check alignment before marking work unit done\nfspec validate-spec-alignment\n\n# If issues found, fix them\nfspec link-coverage user-auth --scenario \"Valid login\" --test-file src/__tests__/auth.test.ts --test-lines 10-25\n\n# Re-validate\nfspec validate-spec-alignment\n\n# Move to done\nfspec update-work-unit-status AUTH-001 done",
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Auto-fix alignment issues",
        example: "# Run with --fix to automatically resolve issues\nfspec validate-spec-alignment --fix\n\n# Review changes\nfspec show-coverage user-auth",
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Validate specific epic",
        example: "# Validate all features in an epic\nfspec list-features --epic AUTH | xargs fspec validate-spec-alignment",
        description: "undefined",
    }),
];

const RELATED: &[&str] = &[
    "validate",
    "link-coverage",
    "show-coverage",
    "audit-coverage",
    "update-work-unit-status",
];

const NOTES: &[&str] = &[
    "Checks that feature scenarios have corresponding test mappings",
    "Verifies that tests have implementation mappings",
    "Ensures scenarios are tagged with work unit IDs for traceability",
    "--fix option attempts to automatically resolve common issues",
    "Exit code 1 if alignment issues found (suitable for CI/CD)",
    "Part of ACDD workflow - run before moving to done state",
];

const PREREQS: &[&str] = &[
    "Feature files must exist in spec/features/",
    "Work units should be tagged in scenarios with @WORK-UNIT-ID",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "validate-spec-alignment",
    description: "Validate alignment between feature specifications, tests, and implementation",
    usage: Some("fspec validate-spec-alignment [feature-files...] [options]"),
    arguments: ARGS,
    options: OPTS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use in the validating phase to ensure specifications, tests, and implementation are aligned. This command checks that scenarios have corresponding tests and implementation, helping maintain ACDD discipline.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQS,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(
        "1. Complete implementation → 2. Link coverage: fspec link-coverage <feature> → 3. Validate alignment: fspec validate-spec-alignment → 4. Fix any issues → 5. Move to done: fspec update-work-unit-status <id> done",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
