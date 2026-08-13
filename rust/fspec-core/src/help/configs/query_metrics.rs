//! `query-metrics` help configuration — Rust port of
//! `src/commands/query-metrics-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js query-metrics --help`
//! when piped to non-TTY. Captured fixture:
//! `rust/fspec/tests/fixtures/help/query-metrics.txt`.
//!
//! NOTE — option drift parity: the TS help config exposes
//! `--metric <metric>` and lists `--format` BEFORE `--work-unit-id`, even
//! though the TS Commander.js action handler at
//! `src/commands/query-metrics.ts:182-198` only reads
//! `--work-unit-id`, `--type`, and `--format`. The fixture is the
//! authoritative byte-stream the Rust binary must reproduce, so this
//! help configuration mirrors the documented (drifted) shape verbatim.

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
        flag: "--metric <metric>",
        description: "Specific metric to query",
        default_value: None,
    },
    CommandOption {
        flag: "--format <format>",
        description: "Output format: table or json",
        default_value: None,
    },
    CommandOption {
        flag: "--work-unit-id <id>",
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
