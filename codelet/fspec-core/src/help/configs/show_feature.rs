//! `show-feature` help configuration — Rust port of
//! `src/commands/show-feature-help.ts` (RPC-304).
//!
//! Byte-for-byte parity with `node dist/index.js show-feature --help`
//! piped to non-TTY. Reference fixture:
//! `codelet/fspec/tests/fixtures/help/show-feature.txt`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig};

const EXAMPLE_1_OUTPUT: &str =
    "Feature: User Login\n  Scenario: Login with valid credentials\n    Given I am on the login page...";

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "file",
    description: "Feature file path",
    required: true,
}];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec show-feature spec/features/login.feature",
    description: Some("Display feature file"),
    output: Some(EXAMPLE_1_OUTPUT),
}];

const RELATED: &[&str] = &["list-features", "validate"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "show-feature",
    description: "Display the contents of a feature file with syntax highlighting",
    usage: Some("fspec show-feature <file>"),
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
