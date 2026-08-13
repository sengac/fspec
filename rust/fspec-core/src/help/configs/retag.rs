//! `retag` help configuration — Rust port of the TS `retag` help
//! registration.
//!
//! Byte-for-byte parity with `node dist/index.js retag --help` piped to
//! non-TTY (see `rust/fspec/tests/fixtures/help/retag.txt`).
//!
//! NOTE on surface: the TS help fixture renders POSITIONAL arguments
//! `<old-tag> <new-tag>` plus a `--dry-run` flag. The clap parsing surface on
//! the standalone binary uses `--from`/`--to`/`--dry-run` flags (see the
//! `cli_retag` integration test SURFACE NOTE), but the rendered `--help`
//! contract is defined by this config and MUST match the captured fixture.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGUMENTS: &[CommandArgument] = &[
    CommandArgument {
        name: "old-tag",
        description: "Tag to replace",
        required: true,
    },
    CommandArgument {
        name: "new-tag",
        description: "New tag name",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "--dry-run",
    description: "Preview changes without making modifications",
    default_value: None,
}];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec retag @wip @in-progress",
        description: Some("Replace @wip with @in-progress in all features"),
        output: Some(
            "✓ Replaced @wip with @in-progress in 5 feature files\n✓ Updated 12 scenarios",
        ),
    },
    CommandExample {
        command: "fspec retag @old @new --dry-run",
        description: Some("Preview tag replacement"),
        output: Some(
            "[DRY RUN] Would replace @old with @new in:\n  - spec/features/login.feature (2 occurrences)\n  - spec/features/checkout.feature (1 occurrence)",
        ),
    },
];

const RELATED: &[&str] = &["register-tag", "update-tag", "validate-tags"];

const NOTES: &[&str] = &[
    "Both tags must be registered in spec/tags.json",
    "Updates both feature-level and scenario-level tags",
    "Run validate-tags after retagging to verify changes",
    "Use --dry-run to preview before making changes",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "retag",
    description: "Replace one tag with another across all feature files",
    usage: Some("fspec retag <old-tag> <new-tag> [options]"),
    arguments: ARGUMENTS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: None,
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: NOTES,
};
