//! `generate-summary-report` help configuration — Rust port of
//! `src/commands/generate-summary-report-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js generate-summary-report --help`.
//! Captured fixture: `codelet/fspec/tests/fixtures/help/generate-summary-report.txt`.

use super::super::{CommandExample, CommandHelpConfig, CommandOption};

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--output <file>",
        description: "Output file path (markdown format)",
        default_value: None,
    },
    CommandOption {
        flag: "--format <format>",
        description: "Output format: markdown, json, or html",
        default_value: None,
    },
];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec generate-summary-report --output report.md",
    description: Some("Generate report"),
    output: Some("✓ Generated summary report: report.md\n  42 work units, 87% complete"),
}];

const RELATED: &[&str] = &["query-metrics", "board", "query-work-units"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "generate-summary-report",
    description: "Generate a comprehensive project summary report",
    usage: Some("fspec generate-summary-report [options]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use to create status reports for stakeholders showing progress, metrics, and health indicators.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: &[],
};
