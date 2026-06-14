//! `add-background` help configuration — Rust port of
//! `src/commands/add-background-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js add-background --help`
//! piped to non-TTY (fixture: codelet/fspec/tests/fixtures/help/add-background.txt).
//!
//! NOTE: the TS `CommonPattern` entries omit a `description` field, so the
//! formatter prints the JS `undefined` string verbatim after each pattern's
//! `Example:` line. We mirror that exactly by setting `description: "undefined"`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommonError, CommonPattern,
    CommonPatternEntry,
};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "feature",
        description: "Feature file name or path (e.g., \"login\" or \"spec/features/login.feature\")",
        required: true,
    },
    CommandArgument {
        name: "text",
        description: "User story text in format: \"As a [role] I want to [action] So that [benefit]\"",
        required: true,
    },
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "User Story Format",
        example: "fspec add-background login \"As a [role]\\nI want to [action]\\nSo that [benefit]\"",
        description: "undefined",
    }),
    CommonPatternEntry::Structured(CommonPattern {
        pattern: "Update Existing Background",
        example: "# Running add-background again replaces the existing Background\nfspec add-background login \"As a premium user\\nI want to login with SSO\\nSo that I can use corporate credentials\"",
        description: "undefined",
    }),
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec add-background login \"As a user\\nI want to login securely\\nSo that I can access my account\"",
        description: Some("Add user story to login feature"),
        output: Some("✓ Added background to login"),
    },
    CommandExample {
        command: "fspec add-background user-auth \"As a developer\\nI want to implement JWT authentication\\nSo that users can access protected resources\"",
        description: Some("Add background to user-auth feature"),
        output: Some("✓ Added background to user-auth"),
    },
    CommandExample {
        command: "fspec add-background spec/features/api.feature \"As an API consumer\\nI want RESTful endpoints\\nSo that I can integrate with the system\"",
        description: Some("Add background using full feature path"),
        output: Some("✓ Added background to spec/features/api.feature"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Error: Background text cannot be empty",
        fix: "Provide text for the Background section as the second argument",
    },
    CommonError {
        error: "Error: Feature file not found: login",
        fix: "Ensure the feature file exists in spec/features/ directory. Create with: fspec create-feature login",
    },
    CommonError {
        error: "Error: Invalid Gherkin syntax in feature file: ...",
        fix: "Feature file has syntax errors. Run: fspec validate spec/features/<feature>.feature",
    },
];

const PREREQUISITES: &[&str] = &["Feature file must exist in spec/features/ directory"];

const RELATED: &[&str] = &["create-feature", "show-feature", "add-scenario", "validate"];

const NOTES: &[&str] = &[
    "Background sections appear before all scenarios in a feature",
    "If a Background already exists, it will be replaced",
    "Background title is always \"User Story\"",
    "Use \\n for multi-line user stories in the command",
    "The feature file is validated before and after modification",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-background",
    description: "Add or update Background (user story) section in a feature file",
    usage: Some("fspec add-background <feature> <text>"),
    arguments: ARGS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some("Use this command to add a Background section containing the user story to a feature file. Background sections provide context for all scenarios in the feature."),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some("1. Create feature: fspec create-feature login → 2. Add background: fspec add-background login \"As a user...\" → 3. Add scenarios → 4. Validate: fspec validate"),
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
