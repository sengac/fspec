//! `query-estimate-accuracy` help configuration — Rust port of
//! `src/commands/query-estimate-accuracy-help.ts` (RPC-258).
//!
//! Byte-for-byte parity with `node dist/index.js query-estimate-accuracy
//! --help` piped to non-TTY. Captured fixture:
//! `codelet/fspec/tests/fixtures/help/query-estimate-accuracy.txt`.
//!
//! Framing note: the TS help advertises `--prefix <prefix>` even though
//! the TS Commander.js registration (lines 164-217 of
//! `src/commands/query-estimate-accuracy.ts`) actually registers only
//! `--format <format>`. We preserve the TS help canon here (the byte
//! fixture asserts the surfaced flag list), and let the standalone clap
//! subcommand bridge expose only the `--format` flag actually accepted
//! by the runtime — matching `show-epic`'s identical framing-A
//! divergence (`codelet/fspec-core/src/help/configs/show_epic.rs`).

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
    flag: "--prefix <prefix>",
    description: "Filter by prefix",
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
