//! `create-feature` help configuration — Rust port of
//! `src/commands/create-feature-help.ts` (RPC-212).
//!
//! Byte-for-byte parity with `node dist/index.js create-feature --help`
//! piped to non-TTY. Reference fixture:
//! `rust/fspec/tests/fixtures/help/create-feature.txt`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig};

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "name",
    description: "Feature name in sentence case (e.g., \"User Authentication\")",
    required: true,
}];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec create-feature \"User Authentication\"",
        description: Some("Create new feature file"),
        output: Some("✓ Created spec/features/user-authentication.feature"),
    },
    CommandExample {
        command: "fspec create-feature \"Payment Processing\"",
        description: Some("Create feature with multi-word name"),
        output: Some("✓ Created spec/features/payment-processing.feature"),
    },
];

const RELATED: &[&str] = &["add-scenario", "add-step", "validate", "format"];

const NOTES: &[&str] = &[
    "Filename is automatically kebab-cased from the feature name",
    "Creates spec/features/ directory if it doesn't exist",
    "Template includes Background section placeholder",
    "Template includes one example Scenario placeholder",
    "File is created with proper Gherkin formatting",
    "Automatically creates corresponding .feature.coverage file for coverage tracking",
    "Detects placeholder text ([role], [action], [benefit]) and provides CLI commands to fix",
    "Returns structured information about prefill detection, coverage file creation, and file naming",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "create-feature",
    description: "Create a new feature file with proper Gherkin structure template",
    usage: Some("fspec create-feature <name>"),
    arguments: ARGUMENTS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use when starting a new feature specification in ACDD workflow. Creates a well-structured template with Background section, placeholder scenarios, and proper formatting.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: NOTES,
};
