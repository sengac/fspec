//! `compact-work-unit` help configuration — Rust port of
//! `src/commands/compact-work-unit-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js compact-work-unit --help`.
//!
//! NOTE: the TS source carries an `aiGuidance` array, but the shared
//! `formatCommandHelp` formatter does NOT render an AI-guidance section, so
//! the captured fixture has no such block. We deliberately omit it here —
//! `CommandHelpConfig` has no `ai_guidance` field.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGS: &[CommandArgument] = &[CommandArgument {
    name: "workUnitId",
    description: "Work unit ID",
    required: true,
}];

const OPTS: &[CommandOption] = &[];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec compact-work-unit AUTH-001",
        description: Some("Compact work unit with confirmation prompt (safe, requires user input)"),
        output: Some(
            "⚠ Warning: This will permanently remove 3 deleted items\nContinue? (y/N) y\n✓ Compacted AUTH-001\nRemoved 3 deleted items, renumbered remaining 7 items to IDs [0-6]",
        ),
    },
];

const RELATED: &[&str] = &["show-deleted", "restore-rule", "update-work-unit-status"];

const NOTES: &[&str] = &[
    "Compaction is PERMANENT and cannot be undone",
    "Removes all soft-deleted items (deleted: true)",
    "Renumbers remaining items sequentially starting from 0",
    "Resets nextId counters to match remaining item count",
    "Preserves chronological order (sorted by createdAt timestamp)",
    "Auto-compact triggers when moving work unit to \"done\" status",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "compact-work-unit",
    description:
        "Permanently remove soft-deleted items and renumber IDs sequentially (destructive operation)",
    usage: Some("fspec compact-work-unit <workUnitId>"),
    arguments: ARGS,
    options: OPTS,
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
