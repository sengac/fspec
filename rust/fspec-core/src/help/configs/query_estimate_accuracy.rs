//! `query-estimate-accuracy` help configuration — Rust port of
//! `src/commands/query-estimate-accuracy-help.ts` (RPC-258).
//!
//! Byte-for-byte parity with `node dist/index.js query-estimate-accuracy
//! --help` piped to non-TTY. Captured fixture:
//! `rust/fspec/tests/fixtures/help/query-estimate-accuracy.txt`.
//!
//! CLI-016: the options match the clap surface (`-f, --format`); the
//! TypeScript-only `--prefix` option is gone.

use super::super::{CommandExample, CommandHelpConfig, CommandOption};

/// Example output rendered directly under the example invocation,
/// matching the TS string literal at `src/commands/query-estimate-accuracy-help.ts`:
///
/// ```text
/// Average estimate accuracy: 87%
/// Underestimated: 5 work units
/// Overestimated: 3 work units
/// ```
const EXAMPLE_OUTPUT: &str =
    "Average estimate accuracy: 87%\nUnderestimated: 5 work units\nOverestimated: 3 work units";

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec query-estimate-accuracy",
    description: Some("Show accuracy metrics"),
    output: Some(EXAMPLE_OUTPUT),
}];

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "-f, --format <format>",
    description: "Output format (text or json)",
    default_value: None,
}];

const RELATED: &[&str] = &["update-work-unit-estimate", "query-metrics"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "query-estimate-accuracy",
    description: "Show estimation accuracy metrics comparing estimates to actuals",
    usage: Some("fspec query-estimate-accuracy [options]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some("Use to assess estimation accuracy and improve future estimates."),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: &[],
};
