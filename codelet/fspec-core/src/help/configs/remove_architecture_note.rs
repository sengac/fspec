//! `remove-architecture-note` help configuration — Rust port of
//! `src/commands/remove-architecture-note-help.ts`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError,
};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "Work unit ID (e.g., WORK-001)",
        required: true,
    },
    CommandArgument {
        name: "index",
        description: "Index of note to remove (0-based, see show-work-unit output)",
        required: true,
    },
];

const OPTS: &[CommandOption] = &[];

const EXAMPLE_1_OUTPUT: &str = "Architecture Notes:\n  0. Uses @cucumber/gherkin parser\n  1. Must complete validation within 2 seconds\n  2. Share validation logic with formatter";
const EXAMPLE_2_OUTPUT: &str = "✓ Architecture note removed successfully";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec show-work-unit WORK-001",
        description: Some("First, view architecture notes with their indices"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec remove-architecture-note WORK-001 1",
        description: Some("Remove note at index 1 (the performance requirement)"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
];

const RELATED: &[&str] = &[
    "show-work-unit",
    "add-architecture-note",
    "generate-scenarios",
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Work unit has no architecture notes",
        fix: "Verify work unit has notes using show-work-unit command",
    },
    CommonError {
        error: "Invalid index N. Work unit has M architecture note(s)",
        fix: "Use show-work-unit to see valid indices (0-based)",
    },
];

const NOTES: &[&str] = &[
    "Indices are 0-based (first note is index 0)",
    "Use show-work-unit to view current notes and their indices",
    "Removing a note will shift subsequent indices down by 1",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "remove-architecture-note",
    description: "Remove architecture note from work unit by index",
    usage: Some("fspec remove-architecture-note <workUnitId> <index>"),
    arguments: ARGS,
    options: OPTS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use when you need to remove an incorrect or outdated architecture note from a work unit. View current notes with show-work-unit to see indices.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
