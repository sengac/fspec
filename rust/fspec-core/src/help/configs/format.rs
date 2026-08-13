//! `format` help configuration — Rust port of `src/commands/format-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js format --help` piped to
//! non-TTY.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig};

const WHEN_TO_USE: &str = "Use to ensure consistent formatting across all feature files. Automatically formats indentation, spacing, and structure according to Gherkin best practices.";

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "file",
    description:
        "Specific feature file to format. If omitted, formats all .feature files in spec/features/",
    required: false,
}];

const EXAMPLE_1_OUTPUT: &str = "✓ Formatted spec/features/login.feature\n✓ Formatted spec/features/signup.feature\n\nFormatted 2 files";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec format",
        description: Some("Format all feature files"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec format spec/features/login.feature",
        description: Some("Format specific file"),
        output: Some("✓ Formatted spec/features/login.feature"),
    },
];

const RELATED: &[&str] = &["validate", "create-feature", "check"];

const NOTES: &[&str] = &[
    "Uses custom AST-based formatter (NOT prettier-plugin-gherkin)",
    "Automatically fixes indentation (2 spaces for scenarios, 4 for steps)",
    "Preserves doc strings (\"\"\") and data tables (|)",
    "Maintains tag formatting",
    "Safe to run multiple times (idempotent)",
    "Run before committing feature files",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "format",
    description: "Format feature files with custom AST-based Gherkin formatter",
    usage: Some("fspec format [file]"),
    arguments: ARGUMENTS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(WHEN_TO_USE),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: NOTES,
};
