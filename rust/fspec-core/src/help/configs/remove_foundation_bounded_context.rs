//! `remove-foundation-bounded-context` help configuration — Rust port of
//! `src/commands/remove-foundation-bounded-context-help.ts`.
//!
//! Byte-for-byte parity with
//! `node dist/index.js remove-foundation-bounded-context --help`.
//! Captured fixture:
//! `rust/fspec/tests/fixtures/help/remove-foundation-bounded-context.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError,
    CommonPatternEntry,
};

const ARGS: &[CommandArgument] = &[CommandArgument {
    name: "context-name",
    description: "Name of the bounded context to remove (e.g., \"Work Management\")",
    required: true,
}];

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "--cascade",
    description:
        "Also remove all child items (aggregates, domain events, commands) within this bounded context",
    default_value: None,
}];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet(
        "Use --cascade when removing contexts that were fully explored but no longer needed",
    ),
    CommonPatternEntry::Bullet(
        "Remove empty contexts first, then re-evaluate children before cascading",
    ),
    CommonPatternEntry::Bullet(
        "Removal uses soft-delete (sets deleted: true) — items are not permanently erased",
    ),
    CommonPatternEntry::Bullet("FOUNDATION.md is auto-regenerated after removal"),
    CommonPatternEntry::Bullet(
        "Combine with show-foundation-event-storm to verify removal results",
    ),
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec remove-foundation-bounded-context \"Legacy System\"",
        description: Some("Remove an empty bounded context"),
        output: Some(
            "✓ Removed bounded context \"Legacy System\" from foundation Event Storm\n✓ Regenerated FOUNDATION.md",
        ),
    },
    CommandExample {
        command: "fspec remove-foundation-bounded-context \"Old Module\" --cascade",
        description: Some(
            "Remove a bounded context and all its child items (aggregates, events, commands)",
        ),
        output: Some(
            "✓ Removed bounded context \"Old Module\" and all its children from foundation Event Storm\n✓ Regenerated FOUNDATION.md",
        ),
    },
    CommandExample {
        command: "fspec remove-foundation-bounded-context \"Work Management\"",
        description: Some("Attempt to remove a non-empty bounded context without --cascade"),
        output: Some(
            "Error: Bounded context 'Work Management' has 5 child items. Use --cascade to remove the context and all its children.",
        ),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Bounded context 'Foo' not found",
        fix: "Check the bounded context name with: fspec show-foundation-event-storm --type bounded-context",
    },
    CommonError {
        error: "has 5 child items. Use --cascade",
        fix: "Add --cascade flag to remove the context and all its aggregates, events, and commands",
    },
    CommonError {
        error: "spec/foundation.json not found",
        fix: "Initialize foundation first: fspec discover-foundation",
    },
];

const PREREQUISITES: &[&str] = &[
    "spec/foundation.json must exist with Event Storm data",
    "The bounded context must exist and not already be deleted",
    "If the context has child items, use --cascade to remove them too",
];

const RELATED: &[&str] = &[
    "add-foundation-bounded-context",
    "remove-aggregate-from-foundation",
    "remove-domain-event-from-foundation",
    "remove-command-from-foundation",
    "show-foundation-event-storm",
];

const NOTES: &[&str] = &[
    "Uses soft-delete (sets deleted: true) consistent with the ItemWithId pattern",
    "Without --cascade, non-empty bounded contexts cannot be removed",
    "--cascade soft-deletes all child items that share the boundedContextId",
    "FOUNDATION.md is automatically regenerated after removal",
    "Soft-deleted items are excluded from show-foundation-event-storm output",
    "Uses fileManager.transaction() for atomic updates",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "remove-foundation-bounded-context",
    description: "Remove a bounded context from foundation Big Picture Event Storm (soft-delete)",
    usage: Some("fspec remove-foundation-bounded-context <context-name> [--cascade]"),
    arguments: ARGS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use to remove a bounded context from the foundation-level Big Picture Event Storm when correcting mistakes or refactoring the domain architecture.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
