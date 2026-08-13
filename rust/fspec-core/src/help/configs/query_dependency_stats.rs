//! `query-dependency-stats` help configuration — Rust port of
//! `src/commands/query-dependency-stats-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js query-dependency-stats --help`
//! when piped to non-TTY. Captured fixture:
//! `rust/fspec/tests/fixtures/help/query-dependency-stats.txt`.

use super::super::{CommandExample, CommandHelpConfig, CommandOption};

/// Example output rendered directly under the example invocation, matching
/// the TS string literal at `src/commands/query-dependency-stats-help.ts`:
///
/// ```text
/// Total dependencies: 42
/// Blocking work units: 5
/// Blocked work units: 8
/// Work units with no dependencies: 20
/// ```
const EXAMPLE_OUTPUT: &str = "Total dependencies: 42\nBlocking work units: 5\nBlocked work units: 8\nWork units with no dependencies: 20";

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "--show-critical-path",
    description: "Highlight critical path through dependencies",
    default_value: None,
}];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec query-dependency-stats",
    description: Some("Show dependency stats"),
    output: Some(EXAMPLE_OUTPUT),
}];

const RELATED: &[&str] = &["export-dependencies", "add-dependency"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "query-dependency-stats",
    description: "Show dependency statistics and potential blockers",
    usage: Some("fspec query-dependency-stats [options]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use to identify dependency bottlenecks, blocked work units, and critical paths.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: &[],
};
