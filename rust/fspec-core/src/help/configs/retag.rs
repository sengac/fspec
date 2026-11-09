//! `retag` help configuration — Rust port of the TS `retag` help
//! registration.
//!
//! Byte-for-byte parity with `node dist/index.js retag --help` piped to
//! non-TTY (see `rust/fspec/tests/fixtures/help/retag.txt`).
//!
//! CLI-016: the help describes the `--from` / `--to` / `--dry-run` flags the
//! CLI parses (the TS help showed positional `<old-tag> <new-tag>`).

use super::super::{CommandExample, CommandHelpConfig, CommandOption};

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--from <tag>",
        description: "Tag to replace",
        default_value: None,
    },
    CommandOption {
        flag: "--to <tag>",
        description: "New tag name",
        default_value: None,
    },
    CommandOption {
        flag: "--dry-run",
        description: "Preview changes without making modifications",
        default_value: None,
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec retag --from=@wip --to=@in-progress",
        description: Some("Replace @wip with @in-progress in all features"),
        output: Some(
            "✓ Replaced @wip with @in-progress in 5 feature files\n✓ Updated 12 scenarios",
        ),
    },
    CommandExample {
        command: "fspec retag --from=@old --to=@new --dry-run",
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
    usage: Some("fspec retag --from <tag> --to <tag> [options]"),
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
    notes: NOTES,
};
