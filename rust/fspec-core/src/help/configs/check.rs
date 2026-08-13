//! `check` help configuration — Rust port of `src/commands/check-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js check --help` piped to
//! non-TTY (captured fixture: `rust/fspec/tests/fixtures/help/check.txt`).
//!
//! Notes on parity:
//!   * The TS source declares NO `options`, so the formatter emits the
//!     `OPTIONS\n  No options available` block. (The runtime `-v/--verbose`
//!     flag is registered on the Commander.js command but absent from the
//!     help config, matching the fixture.)
//!   * `typical_workflow` is a plain string in TS (not an array), so it
//!     renders verbatim on a single line — no comma-join quirk here.
//!   * `related_commands` are bare (no `fspec ` prefix in source), so the
//!     formatter's single prefix yields `fspec validate` etc.

use super::super::{CommandExample, CommandHelpConfig};

const EXAMPLE_1_OUTPUT: &str =
    "✓ Gherkin syntax validation passed\n✓ Tag validation passed\n✓ All files properly formatted\n\n✓ All checks passed";

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec check",
    description: Some("Run all validation checks"),
    output: Some(EXAMPLE_1_OUTPUT),
}];

const PREREQUISITES: &[&str] = &[
    "Feature files must exist in spec/features/",
    "Tags must be registered in spec/tags.json",
];

const RELATED: &[&str] = &["validate", "validate-tags", "format"];

const NOTES: &[&str] = &[
    "Runs validate, validate-tags, and format verification",
    "Exit code 0 = all checks pass, non-zero = failures",
    "Recommended to run before committing",
    "Can be used in pre-commit hooks",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "check",
    description: "Run all validation checks: Gherkin syntax, tag compliance, and formatting",
    usage: Some("fspec check"),
    arguments: &[],
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use before committing or as part of CI/CD pipeline to ensure all specifications meet quality standards. Combines validate, validate-tags, and format checks.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: &[],
    typical_workflow: Some("Edit feature files → fspec check → Fix any issues → Commit"),
    common_errors: &[],
    notes: NOTES,
};
