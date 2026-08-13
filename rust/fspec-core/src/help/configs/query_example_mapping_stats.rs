//! `query-example-mapping-stats` help configuration — Rust port of
//! `src/commands/query-example-mapping-stats-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js query-example-mapping-stats --help`
//! piped to non-TTY. Captured fixture:
//! `rust/fspec/tests/fixtures/help/query-example-mapping-stats.txt`.

use super::super::{CommandExample, CommandHelpConfig, CommandOption};

const EXAMPLE_OUTPUT: &str = "Work units with Example Mapping: 25/42 (59%)\nTotal rules: 87\nTotal examples: 134\nUnanswered questions: 12";

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "--status <status>",
    description: "Filter by status",
    default_value: None,
}];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec query-example-mapping-stats",
    description: Some("Show EM stats"),
    output: Some(EXAMPLE_OUTPUT),
}];

const RELATED: &[&str] = &["show-work-unit", "add-rule", "add-example"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "query-example-mapping-stats",
    description: "Show Example Mapping coverage statistics across work units",
    usage: Some("fspec query-example-mapping-stats [options]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use to assess specification quality and identify work units that need more Example Mapping.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: &[],
};
