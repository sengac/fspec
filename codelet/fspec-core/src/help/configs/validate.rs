//! `validate` help configuration — Rust port of `src/commands/validate-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js validate --help`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError,
};

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "file",
    description: "Specific feature file to validate (e.g., spec/features/login.feature). If omitted, validates all .feature files in spec/features/",
    required: false,
}];

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "-v, --verbose",
    description: "Show detailed validation output including line numbers and suggestions",
    default_value: None,
}];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec validate",
        description: Some("Validate all feature files"),
        output: Some(
            "✓ spec/features/login.feature is valid\n✓ spec/features/signup.feature is valid\n\nValidated 2 files: 2 valid, 0 invalid",
        ),
    },
    CommandExample {
        command: "fspec validate spec/features/login.feature",
        description: Some("Validate specific feature file"),
        output: Some("✓ spec/features/login.feature is valid"),
    },
    CommandExample {
        command: "fspec validate --verbose",
        description: Some("Validate with detailed output"),
        output: Some(
            "✓ spec/features/login.feature is valid\n  - 3 scenarios, 12 steps\n  - Tags: @critical, @authentication",
        ),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "expected: #EOF, #StepLine, got 'And user should see dashboard'",
        fix: "Check indentation - steps must be indented (usually 4 spaces)",
    },
    CommonError {
        error: "expected: #Feature, got 'Scenario'",
        fix: "Feature keyword must come before Scenario. Add Feature: line at top.",
    },
];

const RELATED: &[&str] = &["format", "create-feature", "check", "validate-tags"];

const NOTES: &[&str] = &[
    "Uses official @cucumber/gherkin parser for validation",
    "Exit code 0 = all valid, non-zero = errors found",
    "Run before committing feature files",
    "Part of fspec check command",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "validate",
    description: "Validate Gherkin syntax in feature files using @cucumber/gherkin parser",
    usage: Some("fspec validate [file] [options]"),
    arguments: ARGUMENTS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some("Use this command to verify that your feature files have correct Gherkin syntax before committing. Essential step in ACDD workflow before moving from specifying to testing phase."),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: Some("Write feature file → fspec validate → Fix errors → Repeat until valid → Move to testing phase"),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
