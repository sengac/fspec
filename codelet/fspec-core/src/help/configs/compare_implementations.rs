//! `compare-implementations` help configuration — Rust port of
//! `src/commands/compare-implementations-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js compare-implementations
//! --help` piped to non-TTY.
//!
//! ## TS `undefined`-render bug (intentionally preserved — RPC-207 decision)
//!
//! The TS help config declares its `commonPatterns` entries as
//! `{ title, commands }` objects, but the TS formatter only reads
//! `{ pattern, example, description }`. Each missing field renders the literal
//! string `undefined`, producing two `• undefined` / `Example: undefined` /
//! `undefined` blocks. The fixture captured from the live TS CLI preserves
//! this bug, so the Rust config reproduces it verbatim with three `"undefined"`
//! string fields per structured pattern.

use super::super::{
    CommandExample, CommandHelpConfig, CommandOption, CommonPattern, CommonPatternEntry,
};

const WHEN_TO_USE: &str = "Reviewing implementation consistency across similar features,Identifying naming convention differences,Detecting architectural pattern divergence,Finding opportunities for code reuse or refactoring";

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--tag <tag>",
        description: "Filter work units by tag (e.g., @authentication, @cli)",
        default_value: None,
    },
    CommandOption {
        flag: "--show-coverage",
        description: "Include test and implementation file paths from coverage data",
        default_value: None,
    },
    CommandOption {
        flag: "--json",
        description: "Output results in JSON format",
        default_value: None,
    },
];

// The TS `commonPatterns` were declared with `{ title, commands }` keys that
// the formatter never reads, so each rendered field is the literal
// `undefined`. Preserved intentionally for byte-parity (see module doc).
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

const EXAMPLE_1_OUTPUT: &str = "Comparing 5 work units tagged with @authentication:\n\nAUTH-001: User Login\n  Pattern: JWT-based authentication\n  Naming: camelCase\n\nAUTH-002: OAuth Integration\n  Pattern: Token-based authentication\n  Naming: camelCase\n\n⚠️  Naming Convention Differences:\n  - AUTH-003 uses snake_case instead of camelCase\n  - Consider standardizing on camelCase\n\n✓  Architectural Consistency:\n  - All use token-based auth pattern\n  - Consistent error handling approach";

const EXAMPLE_2_OUTPUT: &str = "CLI-001: Create Feature Command\n  Tests: src/__tests__/create-feature.test.ts\n  Implementation: src/commands/create-feature.ts\n  Pattern: Commander.js with async/await\n\nCLI-002: Validate Command\n  Tests: src/__tests__/validate.test.ts\n  Implementation: src/commands/validate.ts\n  Pattern: Commander.js with async/await\n\n✓  Consistent test and implementation patterns across all CLI commands";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec compare-implementations --tag=@authentication",
        description: Some("Compare authentication implementations"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec compare-implementations --tag=@cli --show-coverage",
        description: Some("Compare with coverage information"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
];

const RELATED: &[&str] = &[
    "search-implementation",
    "show-test-patterns",
    "search-scenarios",
];

const NOTES: &[&str] = &[
    "Automatically detects naming convention differences (camelCase, snake_case, kebab-case)",
    "Highlights architectural pattern divergence",
    "Uses coverage files to analyze actual implementation",
    "Side-by-side comparison helps identify best practices",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "compare-implementations",
    description:
        "Compare implementation approaches across work units to identify patterns and inconsistencies",
    usage: Some("fspec compare-implementations --tag=<tag> [options]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(WHEN_TO_USE),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: &[],
    notes: NOTES,
};
