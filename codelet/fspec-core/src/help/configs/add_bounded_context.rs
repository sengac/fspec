//! `add-bounded-context` help configuration — Rust port of
//! `src/commands/add-bounded-context-help.ts` (RPC-172).
//!
//! Byte-for-byte parity with `node dist/index.js add-bounded-context --help`.
//! Captured fixture: `codelet/fspec/tests/fixtures/help/add-bounded-context.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError,
    CommonPatternEntry,
};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "Work unit ID",
        required: true,
    },
    CommandArgument {
        name: "text",
        description:
            "Bounded context name (e.g., \"Order Management\", \"Inventory\", \"Identity\")",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--description <text>",
        description: "Description of bounded context purpose and scope",
        default_value: None,
    },
    CommandOption {
        flag: "--timestamp <ms>",
        description: "Timeline timestamp in milliseconds (for temporal ordering)",
        default_value: None,
    },
];

const PREREQUISITES: &[&str] = &[
    "Work unit must exist",
    "Work unit must have eventStorm section initialized",
    "Bounded context should represent a clear strategic boundary in the domain",
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet("Bounded contexts represent strategic boundaries in your domain"),
    CommonPatternEntry::Bullet("Each bounded context has its own ubiquitous language"),
    CommonPatternEntry::Bullet("Use bounded contexts to organize aggregates, events, and commands"),
    CommonPatternEntry::Bullet("Bounded contexts communicate through well-defined interfaces"),
    CommonPatternEntry::Bullet(
        "Foundation-level bounded contexts (Big Picture) vs work unit-level (Process Modeling)",
    ),
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec add-bounded-context CHECKOUT-001 \"Order Management\"",
        description: Some("Add bounded context to work unit"),
        output: Some("✓ Added bounded context \"Order Management\" to CHECKOUT-001 (ID: 0)"),
    },
    CommandExample {
        command:
            "fspec add-bounded-context CHECKOUT-001 \"Inventory\" --description \"Manages product stock and warehouse operations\"",
        description: Some("Add bounded context with description"),
        output: Some("✓ Added bounded context \"Inventory\" to CHECKOUT-001 (ID: 1)"),
    },
    CommandExample {
        command:
            "fspec add-bounded-context AUTH-001 \"Identity\" --description \"User authentication and authorization\"",
        description: Some("Add identity bounded context"),
        output: Some("✓ Added bounded context \"Identity\" to AUTH-001 (ID: 0)"),
    },
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Work unit not found",
        fix: "Ensure work unit exists: fspec show-work-unit <id>",
    },
    CommonError {
        error: "Cannot add Event Storm items to work unit in done state",
        fix: "Work unit must be in specifying, testing, implementing, or validating state",
    },
    CommonError {
        error: "spec/work-units.json not found",
        fix: "Initialize fspec first: fspec init",
    },
];

const RELATED: &[&str] = &[
    "add-aggregate",
    "add-domain-event",
    "add-foundation-bounded-context",
    "show-event-storm",
    "show-foundation-event-storm",
];

const NOTES: &[&str] = &[
    "Bounded Contexts are a core DDD pattern for managing complexity",
    "Work unit bounded contexts are tactical (Process Modeling level)",
    "Foundation bounded contexts are strategic (Big Picture level)",
    "Bounded contexts assigned stable IDs starting from 0",
    "Deleted bounded contexts marked with deleted: true (soft delete)",
    "Use show-event-storm to view all Event Storm artifacts",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-bounded-context",
    description:
        "Add bounded context to Event Storm section of work unit for organizing domain boundaries",
    usage: Some("fspec add-bounded-context <workUnitId> <text> [options]"),
    arguments: ARGS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use during Event Storming when identifying strategic boundaries (Bounded Contexts) that separate distinct subdomains or business capabilities in your system.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
