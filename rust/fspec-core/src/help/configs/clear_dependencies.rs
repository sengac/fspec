//! `clear-dependencies` help configuration — Rust port of
//! `src/commands/clear-dependencies-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js clear-dependencies --help`.
//! Captured fixture: `rust/fspec/tests/fixtures/help/clear-dependencies.txt`.
//!
//! NOTE: The TS help text declares a `--force` option even though the
//! Commander.js registration actually uses `--confirm` (see
//! src/commands/clear-dependencies.ts:106). The help fixture is
//! authoritative for parity — we mirror the divergence verbatim.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError,
    CommonPatternEntry,
};

const ARGS: &[CommandArgument] = &[CommandArgument {
    name: "workUnitId",
    description: "Work unit ID",
    required: true,
}];

const OPTS: &[CommandOption] = &[CommandOption {
    flag: "--force",
    description: "Skip confirmation",
    default_value: None,
}];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec clear-dependencies AUTH-001",
    description: Some("Clear all dependencies"),
    output: Some("✓ Cleared all dependencies from AUTH-001"),
}];

const COMMON_ERRORS: &[CommonError] = &[];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[];

const RELATED: &[&str] = &["add-dependency", "remove-dependency"];

const NOTES: &[&str] = &[
    "Removes all relationship types",
    "Requires confirmation unless --force is used",
];

const PREREQS: &[&str] = &[];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "clear-dependencies",
    description: "Remove all dependencies from a work unit",
    usage: Some("fspec clear-dependencies <workUnitId> [options]"),
    arguments: ARGS,
    options: OPTS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: None,
    when_not_to_use: None,
    prerequisites: PREREQS,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
