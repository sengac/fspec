//! `generate-scenarios` help configuration — Rust port of
//! `src/commands/generate-scenarios-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js generate-scenarios --help`.
//! Captured fixture: `rust/fspec/tests/fixtures/help/generate-scenarios.txt`.
//!
//! CLI-016: the options are the `--feature <name>` and
//! `--ignore-possible-duplicates` flags the CLI parses. The
//! EXAMPLES still cite the stale "Generated 3 scenarios" output from before the
//! command became context-only — preserved for byte parity with the captured
//! fixture.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "workUnitId",
    description: "Work unit ID",
    required: true,
}];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--feature <name>",
        description: "Override feature file name (without .feature extension). Defaults to work unit title converted to kebab-case.",
        default_value: None,
    },
    CommandOption {
        flag: "--ignore-possible-duplicates",
        description: "Generate even when similar scenarios exist in other feature files",
        default_value: None,
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec generate-scenarios AUTH-001",
        description: Some("Generate scenarios using work unit title as feature file name"),
        output: Some(
            "✓ Generated 3 scenarios in spec/features/user-authentication.feature\n\nScenario: Login with valid email...\nScenario: Login with invalid email...",
        ),
    },
    CommandExample {
        command: "fspec generate-scenarios AUTH-001 --feature=login",
        description: Some("Generate scenarios with explicit feature file name"),
        output: Some("✓ Generated 3 scenarios in spec/features/login.feature"),
    },
];

const RELATED: &[&str] = &[
    "add-rule",
    "add-example",
    "add-question",
    "show-work-unit",
    "create-feature",
];

const PREREQUISITES: &[&str] = &[
    "Work unit must have rules and examples",
    "All questions should be answered",
    "Work unit must have a title (for default naming) OR --feature flag must be provided",
];

const NOTES: &[&str] = &[
    "Feature files are named after CAPABILITIES (what IS), not work unit IDs",
    "Defaults to work unit title converted to kebab-case (e.g., \"User Authentication\" → user-authentication.feature)",
    "Use --feature flag to override the default name",
    "Throws error if work unit has no title and --feature not provided",
    "Generates scenarios based on examples",
    "Scenario titles are automatically cleaned (removes prefixes like REPRODUCTION:, MISSING:, ERROR WHEN:, etc.)",
    "Original example text is preserved as a comment above each scenario",
    "Uses intelligent step extraction to generate Given/When/Then steps from example text",
    "Falls back to [placeholders] if example cannot be parsed (triggers prefill detection)",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "generate-scenarios",
    description:
        "Generate Gherkin scenarios from Example Mapping data (rules, examples, questions)",
    usage: Some("fspec generate-scenarios <workUnitId> [--feature=<name>]"),
    arguments: ARGUMENTS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use after completing Example Mapping when all questions are answered and ready to create feature file scenarios.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: NOTES,
};
