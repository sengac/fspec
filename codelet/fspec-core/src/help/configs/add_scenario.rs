//! `add-scenario` help configuration — Rust port of
//! `src/commands/add-scenario-help.ts` (RPC-190).
//!
//! Byte-for-byte parity with `node dist/index.js add-scenario --help`
//! piped to non-TTY. Reference fixture:
//! `codelet/fspec/tests/fixtures/help/add-scenario.txt`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig};

const ARGUMENTS: &[CommandArgument] = &[
    CommandArgument {
        name: "file",
        description: "Feature file path (e.g., spec/features/login.feature)",
        required: true,
    },
    CommandArgument {
        name: "scenario-name",
        description: "Name of the scenario (e.g., \"Login with invalid password\")",
        required: true,
    },
];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec add-scenario spec/features/login.feature \"Login with invalid password\"",
    description: Some("Add new scenario"),
    output: Some(
        "✓ Added scenario \"Login with invalid password\" to spec/features/login.feature",
    ),
}];

const RELATED: &[&str] = &["add-step", "create-feature", "delete-scenario"];

const NOTES: &[&str] = &[
    "Creates scenario with placeholder Given/When/Then steps",
    "Scenario is added at the end of the feature file",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-scenario",
    description: "Add a new scenario to an existing feature file",
    usage: Some("fspec add-scenario <file> <scenario-name>"),
    arguments: ARGUMENTS,
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
