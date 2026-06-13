//! `add-aggregate-to-foundation` help configuration — Rust port of
//! `src/commands/add-aggregate-to-foundation-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js
//! add-aggregate-to-foundation --help`. Captured fixture:
//! `codelet/fspec/tests/fixtures/help/add-aggregate-to-foundation.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError,
    CommonPatternEntry,
};

const ARGUMENTS: &[CommandArgument] = &[
    CommandArgument {
        name: "<context-name>",
        description: "Name of the bounded context (must already exist)",
        required: true,
    },
    CommandArgument {
        name: "<aggregate-name>",
        description: "Name of the aggregate to add (e.g., \"WorkUnit\", \"User\", \"Order\")",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "-d, --description <text>",
    description: "Optional description explaining the aggregate's purpose",
    default_value: None,
}];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet("Add aggregates during Big Picture Event Storm sessions"),
    CommonPatternEntry::Bullet("Group related aggregates within same bounded context"),
    CommonPatternEntry::Bullet(
        "Aggregates typically map to core business entities (User, Order, WorkUnit, etc.)",
    ),
    CommonPatternEntry::Bullet("Each aggregate should have clear state and behavior boundaries"),
    CommonPatternEntry::Bullet("Use descriptions to document aggregate responsibilities"),
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec add-aggregate-to-foundation \"Work Management\" \"WorkUnit\"",
        description: Some("Add WorkUnit aggregate to Work Management bounded context"),
        output: Some("✓ Added aggregate \"WorkUnit\" to \"Work Management\" bounded context"),
    },
    CommandExample {
        command: "fspec add-aggregate-to-foundation \"Work Management\" \"WorkUnit\" --description \"Tracks a discrete piece of work\"",
        description: Some("Add aggregate with description"),
        output: Some("✓ Added aggregate \"WorkUnit\" to \"Work Management\" bounded context"),
    },
    CommandExample {
        command: "fspec add-aggregate-to-foundation \"Identity\" \"User\"",
        description: Some("Add User aggregate to Identity bounded context"),
        output: Some("✓ Added aggregate \"User\" to \"Identity\" bounded context"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Bounded context 'Foo' not found",
        fix: "Create the bounded context first: fspec add-foundation-bounded-context \"Foo\"",
    },
    CommonError {
        error: "foundation.json not found",
        fix: "Initialize foundation first: fspec discover-foundation",
    },
];

const PREREQUISITES: &[&str] = &[
    "foundation.json must exist",
    "Big Picture Event Storm must be initialized",
    "Target bounded context must already exist",
];

const RELATED: &[&str] = &[
    "remove-aggregate-from-foundation",
    "add-foundation-bounded-context",
    "add-domain-event-to-foundation",
    "add-command-to-foundation",
    "show-foundation-event-storm",
    "generate-foundation-md",
];

const NOTES: &[&str] = &[
    "Aggregates are stored in foundation.json eventStorm.items array",
    "Each aggregate is linked to a bounded context via boundedContextId",
    "Aggregates have color=\"yellow\" by default (Event Storming convention)",
    "FOUNDATION.md is automatically regenerated after adding aggregate",
    "Foundation aggregates define STRATEGIC model, work unit aggregates are TACTICAL",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-aggregate-to-foundation",
    description: "Add an aggregate to a foundation bounded context in Big Picture Event Storm",
    usage: Some(
        "fspec add-aggregate-to-foundation <context-name> <aggregate-name> [--description <text>]",
    ),
    arguments: ARGUMENTS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use during Big Picture Event Storming to add aggregates (business entities with state and behavior) to specific bounded contexts in the foundation domain model.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
