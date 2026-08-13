//! `search-scenarios` help configuration — Rust port of
//! `src/commands/search-scenarios-help.ts` (RPC-297).
//!
//! Byte-for-byte parity with `node dist/index.js search-scenarios --help`
//! piped to non-TTY. Reference fixture:
//! `rust/fspec/tests/fixtures/help/search-scenarios.txt`.
//!
//! ## Quirk: literal `undefined` in COMMON PATTERNS
//! The TS help config feeds `{title, commands}` objects but the formatter
//! consumes `{pattern, example, description}`, so all three interpolate as
//! the literal text `undefined`. Reproduced verbatim for byte parity.

use super::super::{
    CommandExample, CommandHelpConfig, CommandOption, CommonPattern, CommonPatternEntry,
};

const EXAMPLE_1_OUTPUT: &str = r#"┌────────────────────────────────────────────────────────────┐
│ Scenario                    │ Feature File              │
├────────────────────────────────────────────────────────────┤
│ Validate user input         │ user-registration.feature │
│ Validation error messages   │ form-validation.feature   │
└────────────────────────────────────────────────────────────┘"#;
const EXAMPLE_2_OUTPUT: &str = r#"Found 15 scenarios matching pattern: valid.*
  - Validate user credentials
  - Valid email format check
  - Validation workflow"#;
const EXAMPLE_3_OUTPUT: &str = r#"{
  "scenarios": [
    {
      "name": "Login with valid credentials",
      "featureFile": "spec/features/user-auth.feature",
      "workUnitId": "AUTH-001"
    }
  ]
}"#;

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--query <pattern>",
        description: "Search pattern (literal text or regex)",
        default_value: None,
    },
    CommandOption {
        flag: "--regex",
        description: "Enable regex pattern matching (default: literal)",
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
        command: "fspec search-scenarios --query=\"validation\"",
        description: Some("Find scenarios containing \"validation\""),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec search-scenarios --query=\"valid.*\" --regex",
        description: Some("Find scenarios matching regex pattern"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
    CommandExample {
        command: "fspec search-scenarios --query=\"login\" --json",
        description: Some("Output in JSON format"),
        output: Some(EXAMPLE_3_OUTPUT),
    },
];

const RELATED: &[&str] = &[
    "get-scenarios",
    "search-implementation",
    "compare-implementations",
];

const NOTES: &[&str] = &[
    "Searches scenario names only (not step text)",
    "Regex mode uses JavaScript RegExp syntax",
    "Case-insensitive by default",
    "Results include feature file path and work unit ID for traceability",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "search-scenarios",
    description: "Search for scenarios across all feature files by text or regex pattern",
    usage: Some("fspec search-scenarios --query=<pattern> [options]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some("Finding scenarios with specific keywords across multiple features,Locating test scenarios that mention particular functionality,Searching for scenarios related to a specific component or feature,Pattern-based scenario discovery for refactoring or analysis"),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: &[],
    notes: NOTES,
};
