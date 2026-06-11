//! `add-architecture-note` help configuration — Rust port of
//! `src/commands/add-architecture-note-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js add-architecture-note --help`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonPatternEntry,
};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "Work unit ID (e.g., WORK-001)",
        required: true,
    },
    CommandArgument {
        name: "note",
        description:
            "Architecture note text. Optionally prefix with category (e.g., \"Dependency:\", \"Performance:\", \"Refactoring:\", \"Security:\", \"UI/UX:\", \"Implementation:\")",
        required: true,
    },
];

const OPTS: &[CommandOption] = &[];

const EXAMPLE_OUTPUT_OK: &str = "✓ Architecture note added successfully";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec add-architecture-note WORK-001 \"Uses @cucumber/gherkin parser\"",
        description: Some("Add general architecture note"),
        output: Some(EXAMPLE_OUTPUT_OK),
    },
    CommandExample {
        command:
            "fspec add-architecture-note WORK-001 \"Dependency: @cucumber/gherkin parser\"",
        description: Some("Add dependency note (will be categorized in docstring)"),
        output: Some(EXAMPLE_OUTPUT_OK),
    },
    CommandExample {
        command:
            "fspec add-architecture-note WORK-001 \"Performance: Must complete validation within 2 seconds\"",
        description: Some("Add performance requirement"),
        output: Some(EXAMPLE_OUTPUT_OK),
    },
    CommandExample {
        command:
            "fspec add-architecture-note WORK-001 \"Refactoring: Share validation logic with formatter\"",
        description: Some("Add refactoring note"),
        output: Some(EXAMPLE_OUTPUT_OK),
    },
];

const RELATED: &[&str] = &[
    "show-work-unit",
    "remove-architecture-note",
    "generate-scenarios",
    "add-assumption",
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet("Prefix notes with \"Dependency:\" for libraries and integrations"),
    CommonPatternEntry::Bullet("Prefix with \"Performance:\" for speed/throughput requirements"),
    CommonPatternEntry::Bullet("Prefix with \"Refactoring:\" for code sharing opportunities"),
    CommonPatternEntry::Bullet("Prefix with \"Security:\" for security considerations"),
    CommonPatternEntry::Bullet("Prefix with \"UI/UX:\" for design references or examples"),
    CommonPatternEntry::Bullet("Prefix with \"Implementation:\" for specific patterns to follow"),
    CommonPatternEntry::Bullet("Notes without prefixes go into \"General\" category"),
];

const NOTES: &[&str] = &[
    "Architecture notes are stored in work unit and used to populate feature file docstrings",
    "Notes with recognized prefixes are automatically categorized in generated docstrings",
    "View captured notes with: fspec show-work-unit <workUnitId>",
    "Notes appear in generated feature files when running generate-scenarios",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-architecture-note",
    description: "Add architecture note to work unit during Example Mapping",
    usage: Some("fspec add-architecture-note <workUnitId> <note>"),
    arguments: ARGS,
    options: OPTS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use during Example Mapping (specifying phase) to capture architecture decisions, non-functional requirements, dependencies, performance constraints, security considerations, or implementation patterns that should appear in the generated feature file docstring.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: COMMON_PATTERNS,
    typical_workflow: Some(
        "During Example Mapping: 1) Ask architecture questions, 2) Capture notes with add-architecture-note, 3) Run generate-scenarios to create feature file with populated docstring",
    ),
    common_errors: &[],
    notes: NOTES,
};
