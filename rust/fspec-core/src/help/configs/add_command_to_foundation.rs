//! `add-command-to-foundation` help configuration — Rust port of
//! `src/commands/add-command-to-foundation-help.ts` (RPC-175).
//!
//! Byte-for-byte parity with
//! `node dist/index.js add-command-to-foundation --help`.
//! Captured fixture:
//! `rust/fspec/tests/fixtures/help/add-command-to-foundation.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError,
    CommonPatternEntry,
};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "<context-name>",
        description: "Name of the bounded context (must already exist)",
        required: true,
    },
    CommandArgument {
        name: "<command-name>",
        description: "Name of the command to add (e.g., \"CreateWorkUnit\", \"UpdateStatus\")",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "-d, --description <text>",
    description: "Optional description explaining the command's intent",
    default_value: None,
}];

const PREREQUISITES: &[&str] = &[
    "foundation.json must exist",
    "Big Picture Event Storm must be initialized",
    "Target bounded context must already exist",
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet(
        "Use imperative verb + noun for command names (CreateWorkUnit, UpdateStatus)",
    ),
    CommonPatternEntry::Bullet("Commands represent user intentions or system actions"),
    CommonPatternEntry::Bullet("Add commands during Big Picture Event Storm sessions"),
    CommonPatternEntry::Bullet(
        "Commands typically trigger domain events (CreateWorkUnit → WorkUnitCreated)",
    ),
    CommonPatternEntry::Bullet("Use descriptions to document command purpose and behavior"),
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec add-command-to-foundation \"Work Management\" \"CreateWorkUnit\"",
        description: Some("Add CreateWorkUnit command to Work Management bounded context"),
        output: Some("✓ Added command \"CreateWorkUnit\" to \"Work Management\" bounded context"),
    },
    CommandExample {
        command:
            "fspec add-command-to-foundation \"Work Management\" \"UpdateStatus\" --description \"Changes work unit status\"",
        description: Some("Add command with description"),
        output: Some("✓ Added command \"UpdateStatus\" to \"Work Management\" bounded context"),
    },
    CommandExample {
        command: "fspec add-command-to-foundation \"Identity\" \"AuthenticateUser\"",
        description: Some("Add AuthenticateUser command to Identity bounded context"),
        output: Some("✓ Added command \"AuthenticateUser\" to \"Identity\" bounded context"),
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

const RELATED: &[&str] = &[
    "remove-command-from-foundation",
    "add-foundation-bounded-context",
    "add-aggregate-to-foundation",
    "add-domain-event-to-foundation",
    "show-foundation-event-storm",
    "generate-foundation-md",
];

const NOTES: &[&str] = &[
    "Commands are stored in foundation.json eventStorm.items array",
    "Each command is linked to a bounded context via boundedContextId",
    "Commands have color=\"blue\" by default (Event Storming convention)",
    "FOUNDATION.md is automatically regenerated after adding command",
    "Foundation commands define STRATEGIC model, work unit commands are TACTICAL",
    "Commands should use imperative naming convention (verb + noun)",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-command-to-foundation",
    description: "Add a command to a foundation bounded context in Big Picture Event Storm",
    usage: Some(
        "fspec add-command-to-foundation <context-name> <command-name> [--description <text>]",
    ),
    arguments: ARGS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use during Big Picture Event Storming to add commands (user intentions or actions) to specific bounded contexts in the foundation domain model.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
