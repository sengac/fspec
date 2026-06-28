//! `update-step` help configuration — Rust port of
//! `src/commands/update-step-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js update-step --help`,
//! verified against `codelet/fspec/tests/fixtures/help/update-step.txt`.
//!
//! ## The `undefined` lines
//! The TypeScript `commonPatterns` entries are objects with only `pattern`
//! and `example` fields — they have NO `description`. The TS formatter
//! (`src/utils/help-formatter.ts`) unconditionally emits
//! `    ${p.description}` for structured patterns, so JavaScript renders the
//! missing field as the literal string `undefined`. The Rust
//! `CommonPattern.description` field is non-optional `&'static str`, so we
//! reproduce the quirk verbatim by setting `description: "undefined"`. This
//! is INTENTIONAL parity — the captured fixture contains these `undefined`
//! lines at lines 37, 44, 51.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError, CommonPattern,
    CommonPatternEntry,
};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "feature",
        description:
            "Feature file name or path (e.g., \"user-login\" or \"spec/features/user-login.feature\")",
        required: true,
    },
    CommandArgument {
        name: "scenario",
        description: "Scenario name (must match exactly)",
        required: true,
    },
    CommandArgument {
        name: "current-step",
        description:
            "Current step text to find (with or without keyword, e.g., \"Given I am on the login page\" or \"I am on the login page\")",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--text <text>",
        description: "New step text (without keyword, or with keyword to override)",
        default_value: None,
    },
    CommandOption {
        flag: "--keyword <keyword>",
        description: "New step keyword (Given, When, Then, And, But)",
        default_value: None,
    },
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Refine step wording",
        example:
            "# Original step is unclear\nfspec update-step user-auth \"Login\" \"I log in\" --text \"I submit valid credentials\"\n\n# Verify change\nfspec show-feature user-auth",
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Change step type",
        example:
            "# Convert Given to When for better flow\nfspec update-step checkout \"Purchase\" \"Given I complete checkout\" --keyword When\n\n# Result: \"When I complete checkout\"",
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Batch update multiple steps",
        example:
            "# Update multiple steps in a scenario\nfspec update-step login \"Valid login\" \"I enter email\" --text \"I enter my email address\"\nfspec update-step login \"Valid login\" \"I click login\" --text \"I submit the login form\"\nfspec validate",
        description: "undefined",
    }),
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command:
            "fspec update-step user-login \"Valid user login\" \"Given I am on the login page\" --text \"I navigate to the login page\"",
        description: Some("Update step text, keep keyword"),
        output: Some(
            "✓ Successfully updated step in scenario 'Valid user login' in user-login.feature",
        ),
    },
    CommandExample {
        command:
            "fspec update-step user-login \"Valid user login\" \"Given I am logged out\" --keyword When",
        description: Some("Change step keyword from Given to When"),
        output: Some(
            "✓ Successfully updated step in scenario 'Valid user login' in user-login.feature",
        ),
    },
    CommandExample {
        command:
            "fspec update-step user-login \"Valid user login\" \"I enter credentials\" --text \"I submit the login form\" --keyword When",
        description: Some("Update both text and keyword"),
        output: Some(
            "✓ Successfully updated step in scenario 'Valid user login' in user-login.feature",
        ),
    },
    CommandExample {
        command:
            "fspec update-step spec/features/auth/login.feature \"Login scenario\" \"Given I am logged in\" --text \"the user is authenticated\"",
        description: Some("Update using full path"),
        output: Some(
            "✓ Successfully updated step in scenario 'Login scenario' in login.feature",
        ),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "No updates specified. Use --text and/or --keyword",
        fix: "Provide at least one of --text or --keyword options",
    },
    CommonError {
        error: "Feature file not found: spec/features/user-login.feature",
        fix: "Check feature name or provide full path. Use: fspec list-features",
    },
    CommonError {
        error: "Scenario 'Login scenario' not found in feature file",
        fix: "Verify scenario name matches exactly. Use: fspec show-feature <feature>",
    },
    CommonError {
        error: "Step 'Given I am logged in' not found in scenario 'Login scenario'",
        fix: "Check step text matches exactly (case-sensitive). Use: fspec show-feature <feature>",
    },
    CommonError {
        error: "Update would result in invalid Gherkin: ...",
        fix: "Ensure new step text follows Gherkin syntax rules (e.g., starts with keyword, valid format)",
    },
];

const PREREQUISITES: &[&str] = &[
    "Feature file must exist",
    "Scenario must exist in the feature file",
    "Current step text must match exactly (with or without keyword)",
];

const RELATED: &[&str] = &["add-step", "delete-step", "show-feature", "validate"];

const NOTES: &[&str] = &[
    "Current step can be specified with or without keyword (e.g., \"Given I am logged in\" or \"I am logged in\")",
    "Step matching is case-sensitive and whitespace-sensitive",
    "The command validates that the updated step results in valid Gherkin before saving",
    "Preserves indentation and formatting of the feature file",
    "If --text includes a keyword (e.g., \"When I submit\"), only the text part is used (keyword comes from --keyword)",
    "Must provide at least one of --text or --keyword",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "update-step",
    description:
        "Update step text or keyword in a scenario by finding and replacing the current step",
    usage: Some("fspec update-step <feature> <scenario> <current-step> [options]"),
    arguments: ARGS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command to modify a step in a scenario, either changing its text, keyword (Given/When/Then/And/But), or both. The command finds the step by matching the current text and validates that the result is still valid Gherkin.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(
        "1. View scenario steps: fspec show-feature <feature> → 2. Update step: fspec update-step <feature> <scenario> <current-step> --text <new-text> → 3. Verify change: fspec show-feature <feature> → 4. Validate: fspec validate",
    ),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
