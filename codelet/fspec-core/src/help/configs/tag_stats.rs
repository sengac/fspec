//! `tag-stats` help configuration — Rust port of
//! `src/commands/tag-stats-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js tag-stats --help`
//! when piped to non-TTY. Captured fixture:
//! `codelet/fspec/tests/fixtures/help/tag-stats.txt`.

use super::super::{CommandExample, CommandHelpConfig};

/// Example output rendered directly under the example invocation,
/// matching the TS string literal at `src/commands/tag-stats-help.ts`:
///
/// ```text
/// Tag Usage Statistics:
///
/// @critical: 45 features, 123 scenarios
/// @high: 12 features, 34 scenarios
/// @cli: 30 features
/// @parser: 15 features
/// ```
const EXAMPLE_OUTPUT: &str = "Tag Usage Statistics:\n\n@critical: 45 features, 123 scenarios\n@high: 12 features, 34 scenarios\n@cli: 30 features\n@parser: 15 features";

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec tag-stats",
    description: Some("Show tag usage statistics"),
    output: Some(EXAMPLE_OUTPUT),
}];

const RELATED: &[&str] = &["list-tags", "validate-tags", "register-tag"];

const NOTES: &[&str] = &[
    "Shows how many features and scenarios use each tag",
    "Helps identify unused or underused tags",
    "Only counts tags that are actually used in .feature files",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "tag-stats",
    description: "Display statistics about tag usage across feature files",
    usage: Some("fspec tag-stats"),
    arguments: &[],
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
