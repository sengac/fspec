//! `show-test-patterns` help configuration — Rust port of
//! `src/commands/show-test-patterns-help.ts` (RPC-307).
//!
//! Byte-for-byte parity with `node dist/index.js show-test-patterns --help`
//! piped to non-TTY. Reference fixture:
//! `rust/fspec/tests/fixtures/help/show-test-patterns.txt`.
//!
//! ## Quirk: literal `undefined` in COMMON PATTERNS
//!
//! The TS help config feeds `{title, commands}` objects to the formatter,
//! but the formatter consumes `{pattern, example, description}`. The
//! resulting `undefined.toString()` interpolation surfaces as the literal
//! text `undefined`. We faithfully reproduce that quirk here using
//! `CommonPatternEntry::Structured` with all three fields set to `"undefined"`.

use super::super::{
    CommandExample, CommandHelpConfig, CommandOption, CommonPattern, CommonPatternEntry,
};

const EXAMPLE_1_OUTPUT: &str = "Testing Patterns for @high tagged work units:\n\nCommon Patterns:\n  \u{2713} All use Vitest framework\n  \u{2713} All include integration tests\n  \u{2713} 90% use beforeEach/afterEach setup\n  \u{2713} 85% use custom test utilities\n\nTest Structure:\n  - Average scenarios per feature: 5.2\n  - Average test file size: 250 lines\n  - Coverage: 95% average";

const EXAMPLE_2_OUTPUT: &str = "Testing Patterns for @cli commands:\n\nCLI-001: Create Feature\n  Tests: src/__tests__/create-feature.test.ts (45-120)\n  Pattern: Commander.js + Vitest + temp directories\n\nCLI-002: Validate\n  Tests: src/__tests__/validate.test.ts (23-89)\n  Pattern: Commander.js + Vitest + temp directories\n\n\u{2713}  Consistent Patterns Detected:\n  - All CLI tests use temporary directories for isolation\n  - All use Commander.js for argument parsing\n  - All tests clean up temp files in afterEach";

const EXAMPLE_3_OUTPUT: &str = "{\n  \"tag\": \"@authentication\",\n  \"workUnits\": 5,\n  \"patterns\": {\n    \"framework\": \"Vitest\",\n    \"commonUtilities\": [\"setupTestUser\", \"mockAuth\"],\n    \"averageCoverage\": 92\n  },\n  \"testFiles\": [...]\n}";

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--tag <tag>",
        description: "Filter work units by tag (e.g., @high, @cli)",
        default_value: None,
    },
    CommandOption {
        flag: "--include-coverage",
        description: "Include test file paths and coverage information",
        default_value: None,
    },
    CommandOption {
        flag: "--json",
        description: "Output results in JSON format",
        default_value: None,
    },
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "undefined",
        example: "undefined",
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "undefined",
        example: "undefined",
        description: "undefined",
    }),
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec show-test-patterns --tag=@high",
        description: Some("Show testing patterns for high-priority features"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec show-test-patterns --tag=@cli --include-coverage",
        description: Some("Show patterns with coverage details"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
    CommandExample {
        command: "fspec show-test-patterns --tag=@authentication --json",
        description: Some("Output patterns in JSON format"),
        output: Some(EXAMPLE_3_OUTPUT),
    },
];

const RELATED: &[&str] = &[
    "show-coverage",
    "compare-implementations",
    "search-scenarios",
];

const NOTES: &[&str] = &[
    "Analyzes test files from coverage data",
    "Identifies common testing patterns and utilities",
    "Helps maintain test consistency across similar features",
    "Detects outliers or inconsistent testing approaches",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "show-test-patterns",
    description: "Analyze and display common testing patterns across work units",
    usage: Some("fspec show-test-patterns --tag=<tag> [options]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Identifying common testing patterns across similar features,Ensuring test consistency for work units with same tag,Finding test gaps or inconsistencies,Discovering reusable test utilities or helpers",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: &[],
    notes: NOTES,
};
