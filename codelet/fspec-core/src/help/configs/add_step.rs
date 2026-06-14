//! `add-step` help configuration — Rust port of
//! `src/commands/add-step-help.ts` (RPC-192).
//!
//! Byte-for-byte parity with `node dist/index.js add-step --help`
//! piped to non-TTY. Reference fixture:
//! `codelet/fspec/tests/fixtures/help/add-step.txt`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig};

const ARGUMENTS: &[CommandArgument] = &[
    CommandArgument {
        name: "file",
        description: "Feature file path",
        required: true,
    },
    CommandArgument {
        name: "scenario",
        description: "Scenario name",
        required: true,
    },
    CommandArgument {
        name: "type",
        description: "Step type: given, when, then, and, or but",
        required: true,
    },
    CommandArgument {
        name: "text",
        description: "Step text (e.g., \"I am logged in\")",
        required: true,
    },
];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec add-step spec/features/login.feature \"Login with valid credentials\" given \"I am on the login page\"",
    description: Some("Add given step"),
    output: Some("✓ Added step to scenario"),
}];

const RELATED: &[&str] = &["add-scenario", "update-step", "delete-step"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-step",
    description: "Add a step to a scenario in a feature file",
    usage: Some("fspec add-step <file> <scenario> <type> <text>"),
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
    notes: &[],
};
