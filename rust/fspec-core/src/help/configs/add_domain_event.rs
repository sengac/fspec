//! `add-domain-event` help configuration — Rust port of
//! `src/commands/add-domain-event-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js add-domain-event --help`
//! piped to non-TTY. Captured fixture:
//! `rust/fspec/tests/fixtures/help/add-domain-event.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandHelpConfig, CommandOption, CommonError,
    CommonPatternEntry,
};

const ARGUMENTS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "Work unit ID",
        required: true,
    },
    CommandArgument {
        name: "text",
        description:
            "Event text/name (PascalCase recommended, e.g., \"UserRegistered\", \"OrderPlaced\")",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--timestamp <ms>",
        description: "Timeline timestamp in milliseconds (for temporal ordering of events)",
        default_value: None,
    },
    CommandOption {
        flag: "--bounded-context <context>",
        description: "Bounded context for domain association (DDD concept)",
        default_value: None,
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec add-domain-event AUTH-001 \"UserRegistered\"",
        description: Some("Add domain event to work unit"),
        output: Some("✓ Added domain event \"UserRegistered\" to AUTH-001 (ID: 0)"),
    },
    CommandExample {
        command:
            "fspec add-domain-event AUTH-001 \"UserAuthenticated\" --bounded-context \"Identity\"",
        description: Some("Add domain event with bounded context"),
        output: Some("✓ Added domain event \"UserAuthenticated\" to AUTH-001 (ID: 1)"),
    },
    CommandExample {
        command: "fspec add-domain-event CHECKOUT-001 \"OrderPlaced\" --timestamp 1000",
        description: Some("Add domain event with timeline timestamp"),
        output: Some("✓ Added domain event \"OrderPlaced\" to CHECKOUT-001 (ID: 0)"),
    },
];

const RELATED: &[&str] = &[
    "add-command",
    "add-policy",
    "add-hotspot",
    "show-event-storm",
    "show-foundation-event-storm",
    "generate-example-mapping-from-event-storm",
];

const PREREQUISITES: &[&str] = &[
    "Work unit must exist",
    "Work unit must have eventStorm section initialized",
    "Event text should describe WHAT HAPPENED (past tense)",
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet("Use PascalCase for event names (UserRegistered, OrderPlaced)"),
    CommonPatternEntry::Bullet("Events represent WHAT HAPPENED (past tense, not imperative)"),
    CommonPatternEntry::Bullet("Group related events using --bounded-context flag"),
    CommonPatternEntry::Bullet(
        "Use --timestamp for temporal ordering on Big Picture Event Storm timeline",
    ),
    CommonPatternEntry::Bullet(
        "Generate Example Mapping from events: fspec generate-example-mapping-from-event-storm",
    ),
];

const COMMON_ERRORS: &[CommonError] = &[
    CommonError {
        error: "Work unit not found",
        fix: "Ensure work unit exists: fspec show-work-unit <id>",
    },
    CommonError {
        error: "spec/work-units.json not found",
        fix: "Initialize fspec first: fspec init",
    },
];

const NOTES: &[&str] = &[
    "Domain events are immutable facts that occurred in the system",
    "Events assigned stable IDs starting from 0",
    "Deleted events marked with deleted: true (soft delete)",
    "Use show-event-storm to view all Event Storm artifacts",
    "Bounded contexts help organize events by subdomain (DDD pattern)",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-domain-event",
    description:
        "Add domain event to Event Storm section of work unit for Big Picture Event Storming",
    usage: Some("fspec add-domain-event <workUnitId> <text> [options]"),
    arguments: ARGUMENTS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use during Big Picture Event Storming discovery phase when capturing significant domain events that represent state changes in the system.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
