//! `show-foundation` help configuration — Rust port of
//! `src/commands/show-foundation-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js show-foundation --help`
//! piped to non-TTY. Captured fixture:
//! `codelet/fspec/tests/fixtures/help/show-foundation.txt`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const EXAMPLE_1_OUTPUT: &str =
    "{\n  \"projectName\": \"fspec\",\n  \"version\": \"1.0.0\",\n  \"sections\": {...}\n}";
const EXAMPLE_2_OUTPUT: &str =
    "Available sections:\n  - Architecture Diagrams\n  - System Overview\n  - Project Goals";
const EXAMPLE_3_OUTPUT: &str =
    "{\n  \"Architecture Diagrams\": [\n    {...}\n  ]\n}";
const EXAMPLE_4_OUTPUT: &str =
    "1: {\n2:   \"System Overview\": {\n3:     ...\n4:   }\n5: }";

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "section",
    description: "Specific section to show (optional)",
    required: false,
}];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--list-sections",
        description: "List section names only (without content)",
        default_value: None,
    },
    CommandOption {
        flag: "--line-numbers",
        description: "Show line numbers in output",
        default_value: None,
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec show-foundation",
        description: Some("Show all foundation data"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec show-foundation --list-sections",
        description: Some("List available sections"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
    CommandExample {
        command: "fspec show-foundation \"Architecture Diagrams\"",
        description: Some("Show specific section"),
        output: Some(EXAMPLE_3_OUTPUT),
    },
    CommandExample {
        command: "fspec show-foundation \"System Overview\" --line-numbers",
        description: Some("Show section with line numbers"),
        output: Some(EXAMPLE_4_OUTPUT),
    },
];

const RELATED: &[&str] = &["update-foundation", "add-diagram", "validate-foundation-schema"];

const NOTES: &[&str] = &[
    "foundation.json is the machine-readable source of truth",
    "Use generate-foundation-md for human-readable output",
    "--list-sections helps discover available section names",
    "--line-numbers useful for debugging or referencing specific lines",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "show-foundation",
    description: "Display contents of foundation.json",
    usage: Some("fspec show-foundation [section] [options]"),
    arguments: ARGUMENTS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: None,
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: NOTES,
};
