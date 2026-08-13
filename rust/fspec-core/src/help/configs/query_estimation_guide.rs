//! `query-estimation-guide` help configuration — Rust port of
//! `src/commands/query-estimation-guide-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js query-estimation-guide --help`
//! when piped to non-TTY. Captured fixture:
//! `rust/fspec/tests/fixtures/help/query-estimation-guide.txt`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const EX1_OUTPUT: &str = "Based on 5 similar work units:\n  Suggested estimate: 5 points\n  Range: 3-8 points\n  Average actual: 5.2 points";

const ARGUMENTS: &[CommandArgument] = &[CommandArgument {
    name: "workUnitId",
    description: "Work unit ID",
    required: true,
}];

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "--similar <count>",
    description: "Number of similar work units to analyze",
    default_value: None,
}];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec query-estimation-guide AUTH-001",
    description: Some("Get estimation guidance"),
    output: Some(EX1_OUTPUT),
}];

const RELATED: &[&str] = &["update-work-unit-estimate", "query-estimate-accuracy"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "query-estimation-guide",
    description: "Get estimation guidance based on historical data for a work unit",
    usage: Some("fspec query-estimation-guide <workUnitId> [options]"),
    arguments: ARGUMENTS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use after generating scenarios from Example Mapping to get data-driven estimation guidance based on similar historical work.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: &[],
};
