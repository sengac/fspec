//! `remove-domain-event-from-foundation` help configuration — Rust port of
//! `src/commands/remove-domain-event-from-foundation-help.ts` (RPC-272).
//!
//! Byte-for-byte parity with
//! `node dist/index.js remove-domain-event-from-foundation --help`.
//! Captured fixture:
//! `rust/fspec/tests/fixtures/help/remove-domain-event-from-foundation.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError,
    CommonPatternEntry,
};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "<context-name>",
        description: "Name of the bounded context containing the domain event",
        required: true,
    },
    CommandArgument {
        name: "<event-name>",
        description:
            "Name of the domain event to remove (e.g., \"WorkUnitCreated\", \"UserLoggedIn\")",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[];

const PREREQUISITES: &[&str] = &[
    "spec/foundation.json must exist with Event Storm data",
    "The target bounded context must exist and not be deleted",
    "The domain event must exist within the specified bounded context",
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet(
        "Requires both context name and event name (matching the add command signature)",
    ),
    CommonPatternEntry::Bullet(
        "Uses soft-delete (sets deleted: true) — items are not permanently erased",
    ),
    CommonPatternEntry::Bullet("FOUNDATION.md is auto-regenerated after removal"),
    CommonPatternEntry::Bullet(
        "Only removes the domain event — other items in the context are unaffected",
    ),
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec remove-domain-event-from-foundation \"Work Management\" \"LegacyEventFired\"",
        description: Some("Remove a domain event from a bounded context"),
        output: Some(
            "✓ Removed domain event \"LegacyEventFired\" from \"Work Management\" bounded context\n✓ Regenerated FOUNDATION.md",
        ),
    },
    CommandExample {
        command: "fspec remove-domain-event-from-foundation \"Identity\" \"OldUserLoggedIn\"",
        description: Some("Remove domain event from Identity bounded context"),
        output: Some(
            "✓ Removed domain event \"OldUserLoggedIn\" from \"Identity\" bounded context\n✓ Regenerated FOUNDATION.md",
        ),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Bounded context 'Foo' not found",
        fix: "Check context names with: fspec show-foundation-event-storm --type bounded-context",
    },
    CommonError {
        error: "Domain event 'Bar' not found in bounded context 'Foo'",
        fix: "Check event names with: fspec show-foundation-event-storm",
    },
    CommonError {
        error: "spec/foundation.json not found",
        fix: "Initialize foundation first: fspec discover-foundation",
    },
];

const RELATED: &[&str] = &[
    "add-domain-event-to-foundation",
    "remove-foundation-bounded-context",
    "remove-aggregate-from-foundation",
    "remove-command-from-foundation",
    "show-foundation-event-storm",
];

const NOTES: &[&str] = &[
    "Uses soft-delete (sets deleted: true) consistent with the ItemWithId pattern",
    "Both the bounded context and domain event must exist and not be deleted",
    "FOUNDATION.md is automatically regenerated after removal",
    "Uses fileManager.transaction() for atomic updates",
    "Domain events are identified by name within their bounded context",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "remove-domain-event-from-foundation",
    description: "Remove a domain event from a foundation bounded context (soft-delete)",
    usage: Some("fspec remove-domain-event-from-foundation <context-name> <event-name>"),
    arguments: ARGS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use to remove a domain event from a bounded context in the foundation Big Picture Event Storm when correcting mistakes or refactoring the domain model.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
