//! `show-coverage` help configuration — Rust port of
//! `src/commands/show-coverage-help.ts` (RPC-300).
//!
//! Byte-for-byte parity with `node dist/index.js show-coverage --help`
//! piped to non-TTY. Reference fixture:
//! `codelet/fspec/tests/fixtures/help/show-coverage.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError, CommonPattern,
    CommonPatternEntry,
};

const EX1_OUT: &str = "Coverage Report: user-authentication.feature\nCoverage: 50% (1/2 scenarios)\n\n## Scenarios\n### ✅ Login with valid credentials (FULLY COVERED)\n- **Test**: src/__tests__/auth.test.ts:45-62\n- **Implementation**: src/auth/login.ts:10,11,12,23,24\n\n### ❌ Login with invalid credentials (NOT COVERED)\n- No test mappings";

const EX2_OUT: &str = "Project Coverage Report\nOverall Coverage: 65% (13/20 scenarios)\n\nFeatures Overview:\n- user-authentication.feature: 50% (1/2) ⚠️\n- user-logout.feature: 100% (1/1) ✅\n- dashboard.feature: 67% (2/3) ⚠️\n- admin-tools.feature: 100% (4/4) ✅";

const EX3_OUT: &str = "{\n  \"feature\": \"user-authentication\",\n  \"totalScenarios\": 2,\n  \"coveredScenarios\": 1,\n  \"coveragePercent\": 50,\n  \"scenarios\": [...]\n}";

const EX4_OUT: &str = "✓ Coverage report written to coverage-report.txt";

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "feature-name",
    description: "Feature file name (without path or extension), e.g., \"user-authentication\". If omitted, shows project-wide coverage.",
    required: false,
}];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--format <format>",
        description: "Output format: text or json (default: text)",
        default_value: None,
    },
    CommandOption {
        flag: "--output <file>",
        description: "Write output to file instead of stdout",
        default_value: None,
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec show-coverage user-authentication",
        description: Some("Show coverage for specific feature"),
        output: Some(EX1_OUT),
    },
    CommandExample {
        command: "fspec show-coverage",
        description: Some("Show project-wide coverage report"),
        output: Some(EX2_OUT),
    },
    CommandExample {
        command: "fspec show-coverage user-authentication --format=json",
        description: Some("Get coverage in JSON format for programmatic access"),
        output: Some(EX3_OUT),
    },
    CommandExample {
        command: "fspec show-coverage --output=coverage-report.txt",
        description: Some("Save coverage report to file"),
        output: Some(EX4_OUT),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Error: Feature file user-authentication.feature not found",
        fix: "Check feature name matches file name. Run: fspec list-features",
    },
    CommonError {
        error: "Error: Coverage file user-authentication.feature.coverage not found",
        fix: "Coverage file should auto-create with fspec create-feature. Try running create-feature again.",
    },
];

const PATTERN_1_EXAMPLE: &str = "# See project-wide coverage\nfspec show-coverage\n\n# Focus on specific feature\nfspec show-coverage user-authentication\n\n# Identify scenarios marked ❌ NOT COVERED\n# Link missing tests/implementation";

const PATTERN_2_EXAMPLE: &str = "# Check what percentage of scenarios are mapped\nfspec show-coverage\n# Output: Overall Coverage: 20% (4/20 scenarios)\n\n# Map more scenarios...\nfspec link-coverage ...\n\n# Check progress\nfspec show-coverage\n# Output: Overall Coverage: 45% (9/20 scenarios)";

const PATTERN_3_EXAMPLE: &str = "# Export as JSON for automated checks\nfspec show-coverage --format=json --output=coverage.json\n\n# Parse in CI pipeline to enforce coverage thresholds\njq '.coveragePercent' coverage.json";

// The TS reference output emits the literal string "undefined" for the
// CommonPattern.description field (because show-coverage-help.ts omits it
// and the TS formatter prints the missing field as the JS string
// "undefined"). We reproduce this faithfully for byte-parity.
const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Find Coverage Gaps",
        example: PATTERN_1_EXAMPLE,
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Reverse ACDD Progress Tracking",
        example: PATTERN_2_EXAMPLE,
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Export Coverage for CI/CD",
        example: PATTERN_3_EXAMPLE,
        description: "undefined",
    }),
];

const RELATED: &[&str] = &[
    "link-coverage",
    "audit-coverage",
    "unlink-coverage",
    "list-features",
];

const NOTES: &[&str] = &[
    "Coverage symbols: ✅ FULLY COVERED, ⚠️ PARTIALLY COVERED (test only), ❌ NOT COVERED",
    "Project-wide report (no args) shows overall coverage percentage",
    "Use --format=json for programmatic access in CI/CD pipelines",
    "Coverage percentages based on scenarios, not lines of code",
    "Run regularly to track reverse ACDD mapping progress",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "show-coverage",
    description: "Display coverage report showing scenario-to-test-to-implementation traceability",
    usage: Some("fspec show-coverage [feature-name] [options]"),
    arguments: ARGUMENTS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some("Use this command to view coverage status for features, identify uncovered scenarios (gaps), verify traceability links, and track reverse ACDD mapping progress. Run regularly to find scenarios that need test/implementation links."),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some("1. fspec show-coverage → 2. Identify uncovered scenarios (gaps) → 3. Link tests/implementation → 4. Verify coverage increased"),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
