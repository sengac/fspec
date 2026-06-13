//! `remove-command-from-foundation` help configuration — Rust port of
//! `src/commands/remove-command-from-foundation-help.ts` (RPC-270).
//!
//! Byte-for-byte parity with
//! `node dist/index.js remove-command-from-foundation --help`.
//! Captured fixture:
//! `codelet/fspec/tests/fixtures/help/remove-command-from-foundation.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError,
    CommonPatternEntry,
};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "<context-name>",
        description: "Name of the bounded context containing the command",
        required: true,
    },
    CommandArgument {
        name: "<command-name>",
        description: "Name of the command to remove (e.g., \"CreateWorkUnit\", \"UpdateStatus\")",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[];

const PREREQUISITES: &[&str] = &[
    "spec/foundation.json must exist with Event Storm data",
    "The target bounded context must exist and not be deleted",
    "The command must exist within the specified bounded context",
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet(
        "Requires both context name and command name (matching the add command signature)",
    ),
    CommonPatternEntry::Bullet(
        "Uses soft-delete (sets deleted: true) — items are not permanently erased",
    ),
    CommonPatternEntry::Bullet("FOUNDATION.md is auto-regenerated after removal"),
    CommonPatternEntry::Bullet(
        "Only removes the command — other items in the context are unaffected",
    ),
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec remove-command-from-foundation \"Work Management\" \"DeprecatedAction\"",
        description: Some("Remove a command from a bounded context"),
        output: Some(
            "✓ Removed command \"DeprecatedAction\" from \"Work Management\" bounded context\n✓ Regenerated FOUNDATION.md",
        ),
    },
    CommandExample {
        command: "fspec remove-command-from-foundation \"Identity\" \"OldAuthenticateUser\"",
        description: Some("Remove command from Identity bounded context"),
        output: Some(
            "✓ Removed command \"OldAuthenticateUser\" from \"Identity\" bounded context\n✓ Regenerated FOUNDATION.md",
        ),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Bounded context 'Foo' not found",
        fix: "Check context names with: fspec show-foundation-event-storm --type bounded-context",
    },
    CommonError {
        error: "Command 'Bar' not found in bounded context 'Foo'",
        fix: "Check command names with: fspec show-foundation-event-storm",
    },
    CommonError {
        error: "spec/foundation.json not found",
        fix: "Initialize foundation first: fspec discover-foundation",
    },
];

const RELATED: &[&str] = &[
    "add-command-to-foundation",
    "remove-foundation-bounded-context",
    "remove-aggregate-from-foundation",
    "remove-domain-event-from-foundation",
    "show-foundation-event-storm",
];

const NOTES: &[&str] = &[
    "Uses soft-delete (sets deleted: true) consistent with the ItemWithId pattern",
    "Both the bounded context and command must exist and not be deleted",
    "FOUNDATION.md is automatically regenerated after removal",
    "Uses fileManager.transaction() for atomic updates",
    "Commands are identified by name within their bounded context",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "remove-command-from-foundation",
    description: "Remove a command from a foundation bounded context (soft-delete)",
    usage: Some("fspec remove-command-from-foundation <context-name> <command-name>"),
    arguments: ARGS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use to remove a command from a bounded context in the foundation Big Picture Event Storm when correcting mistakes or refactoring the domain model.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
