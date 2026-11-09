//! `query-metrics` help configuration — Rust port of
//! `src/commands/query-metrics-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js query-metrics --help`
//! when piped to non-TTY. Captured fixture:
//! `rust/fspec/tests/fixtures/help/query-metrics.txt`.
//!
//! CLI-016: the options match the clap surface; the TypeScript-only
//! `--metric` option is gone.

use super::super::{CommandExample, CommandHelpConfig, CommandOption};

/// Example output rendered directly under the example invocation, matching
/// the TS string literal at `src/commands/query-metrics-help.ts`.
const EXAMPLE_OUTPUT: &str =
    "Velocity: 23 points/week\nCycle time: 3.5 days avg\nThroughput: 5 work units/week";

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec query-metrics",
    description: Some("Show all metrics"),
    output: Some(EXAMPLE_OUTPUT),
}];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--format <format>",
        description: "Output format: table or json",
        default_value: None,
    },
    CommandOption {
        flag: "--work-unit <workUnitId>",
        description: "Specific work unit to query metrics for",
        default_value: None,
    },
];

const RELATED: &[&str] = &["generate-summary-report"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "query-metrics",
    description: "Query project metrics and statistics",
    usage: Some("fspec query-metrics [options]"),
    arguments: &[],
    options: OPTIONS,
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
