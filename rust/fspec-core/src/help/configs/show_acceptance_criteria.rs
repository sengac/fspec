//! `show-acceptance-criteria` help configuration — Rust port of
//! `src/commands/show-acceptance-criteria-help.ts` (RPC-299).
//!
//! Byte-for-byte parity with `node dist/index.js show-acceptance-criteria --help`
//! piped to non-TTY. Reference fixture:
//! `rust/fspec/tests/fixtures/help/show-acceptance-criteria.txt`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig};

const EXAMPLE_1_OUTPUT: &str = "Feature: User Login\n\nScenario: Login with valid credentials\n  Given I am on the login page\n  When I enter valid credentials\n  Then I should be logged in";

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "file",
    description: "Feature file path",
    required: true,
}];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec show-acceptance-criteria spec/features/login.feature",
    description: Some("Show acceptance criteria"),
    output: Some(EXAMPLE_1_OUTPUT),
}];

const RELATED: &[&str] = &["get-scenarios", "show-feature"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "show-acceptance-criteria",
    description: "Display acceptance criteria (scenarios) for a feature file",
    usage: Some("fspec show-acceptance-criteria <file>"),
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
