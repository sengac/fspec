//! `add-architecture` help configuration — Rust port of
//! `src/commands/add-architecture-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js add-architecture --help`
//! piped to non-TTY (fixture: rust/fspec/tests/fixtures/help/add-architecture.txt).
//!
//! This is the minimal help shape — no when-to-use, prerequisites, common
//! patterns, typical workflow, or common errors.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "file",
        description: "Feature file path",
        required: true,
    },
    CommandArgument {
        name: "notes",
        description: "Architecture notes text",
        required: true,
    },
];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command:
        "fspec add-architecture spec/features/login.feature \"Uses bcrypt for password hashing\"",
    description: Some("Add architecture notes"),
    output: Some("✓ Added architecture notes to spec/features/login.feature"),
}];

const RELATED: &[&str] = &["create-feature", "show-feature"];

const NOTES: &[&str] = &[
    "Notes are added as doc strings (\"\"\") after Feature line",
    "Use for technical implementation details",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-architecture",
    description: "Add architecture notes to a feature file using doc strings",
    usage: Some("fspec add-architecture <file> <notes>"),
    arguments: ARGS,
    options: &[],
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
