//! `remove-aggregate-from-foundation` help configuration — Rust port of
//! `src/commands/remove-aggregate-from-foundation-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js
//! remove-aggregate-from-foundation --help`. Captured fixture:
//! `rust/fspec/tests/fixtures/help/remove-aggregate-from-foundation.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommonError, CommonPatternEntry,
};

const ARGUMENTS: &[CommandArgument] = &[
    CommandArgument {
        name: "<context-name>",
        description: "Name of the bounded context containing the aggregate",
        required: true,
    },
    CommandArgument {
        name: "<aggregate-name>",
        description: "Name of the aggregate to remove (e.g., \"WorkUnit\", \"User\")",
        required: true,
    },
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet(
        "Requires both context name and aggregate name (matching the add command signature)",
    ),
    CommonPatternEntry::Bullet(
        "Uses soft-delete (sets deleted: true) — items are not permanently erased",
    ),
    CommonPatternEntry::Bullet("FOUNDATION.md is auto-regenerated after removal"),
    CommonPatternEntry::Bullet(
        "Only removes the aggregate — other items in the context are unaffected",
    ),
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec remove-aggregate-from-foundation \"Work Management\" \"LegacyItem\"",
        description: Some("Remove an aggregate from a bounded context"),
        output: Some(
            "✓ Removed aggregate \"LegacyItem\" from \"Work Management\" bounded context\n✓ Regenerated FOUNDATION.md",
        ),
    },
    CommandExample {
        command: "fspec remove-aggregate-from-foundation \"Identity\" \"OldUser\"",
        description: Some("Remove aggregate from Identity bounded context"),
        output: Some(
            "✓ Removed aggregate \"OldUser\" from \"Identity\" bounded context\n✓ Regenerated FOUNDATION.md",
        ),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Bounded context 'Foo' not found",
        fix: "Check context names with: fspec show-foundation-event-storm --type bounded-context",
    },
    CommonError {
        error: "Aggregate 'Bar' not found in bounded context 'Foo'",
        fix: "Check aggregate names with: fspec show-foundation-event-storm",
    },
    CommonError {
        error: "spec/foundation.json not found",
        fix: "Initialize foundation first: fspec discover-foundation",
    },
];

const PREREQUISITES: &[&str] = &[
    "spec/foundation.json must exist with Event Storm data",
    "The target bounded context must exist and not be deleted",
    "The aggregate must exist within the specified bounded context",
];

const RELATED: &[&str] = &[
    "add-aggregate-to-foundation",
    "remove-foundation-bounded-context",
    "remove-domain-event-from-foundation",
    "remove-command-from-foundation",
    "show-foundation-event-storm",
];

const NOTES: &[&str] = &[
    "Uses soft-delete (sets deleted: true) consistent with the ItemWithId pattern",
    "Both the bounded context and aggregate must exist and not be deleted",
    "FOUNDATION.md is automatically regenerated after removal",
    "Uses fileManager.transaction() for atomic updates",
    "Aggregates are identified by name within their bounded context",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "remove-aggregate-from-foundation",
    description: "Remove an aggregate from a foundation bounded context (soft-delete)",
    usage: Some("fspec remove-aggregate-from-foundation <context-name> <aggregate-name>"),
    arguments: ARGUMENTS,
    options: &[],
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use to remove an aggregate from a bounded context in the foundation Big Picture Event Storm when correcting mistakes or refactoring the domain model.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
