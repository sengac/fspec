//! `add-external-system` help configuration — Rust port of
//! `src/commands/add-external-system-help.ts` (RPC-182).
//!
//! Byte-for-byte parity with `node dist/index.js add-external-system --help`.
//! Captured fixture: `codelet/fspec/tests/fixtures/help/add-external-system.txt`.

use super::super::{
    CommandArgument, CommandExample, CommandOption, CommonError, CommonPatternEntry,
    CommandHelpConfig,
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
            "External system name (e.g., \"Payment Gateway\", \"Email Service\", \"User Database\")",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--type <type>",
        description:
            "External system type: REST_API, MESSAGE_QUEUE, DATABASE, THIRD_PARTY_SERVICE, FILE_SYSTEM",
        default_value: None,
    },
    CommandOption {
        flag: "--timestamp <ms>",
        description: "Timeline timestamp in milliseconds (for temporal ordering)",
        default_value: None,
    },
    CommandOption {
        flag: "--bounded-context <context>",
        description: "Bounded context for domain association",
        default_value: None,
    },
];

const PREREQUISITES: &[&str] = &[
    "Work unit must exist",
    "Work unit must have eventStorm section initialized",
    "External system should represent a clear external dependency",
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet("External systems help identify integration points and dependencies"),
    CommonPatternEntry::Bullet("Use --type to categorize external systems for better documentation"),
    CommonPatternEntry::Bullet(
        "External systems represent boundaries between your domain and the outside world",
    ),
    CommonPatternEntry::Bullet("Consider anti-corruption layers when integrating external systems"),
    CommonPatternEntry::Bullet("External systems have pink sticky notes in physical Event Storming"),
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec add-external-system CHECKOUT-001 \"Payment Gateway\"",
        description: Some("Add external system to work unit"),
        output: Some("✓ Added external system \"Payment Gateway\" to CHECKOUT-001 (ID: 0)"),
    },
    CommandExample {
        command: "fspec add-external-system CHECKOUT-001 \"Stripe API\" --type REST_API",
        description: Some("Add external system with type"),
        output: Some("✓ Added external system \"Stripe API\" to CHECKOUT-001 (ID: 1)"),
    },
    CommandExample {
        command:
            "fspec add-external-system AUTH-001 \"User Database\" --type DATABASE --bounded-context \"Identity\"",
        description: Some("Add external system with type and bounded context"),
        output: Some("✓ Added external system \"User Database\" to AUTH-001 (ID: 0)"),
    },
    CommandExample {
        command: "fspec add-external-system NOTIFY-001 \"RabbitMQ\" --type MESSAGE_QUEUE",
        description: Some("Add message queue external system"),
        output: Some("✓ Added external system \"RabbitMQ\" to NOTIFY-001 (ID: 0)"),
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
        error: "Invalid external system type",
        fix: "Use one of: REST_API, MESSAGE_QUEUE, DATABASE, THIRD_PARTY_SERVICE, FILE_SYSTEM",
    },
    CommonError {
        error: "spec/work-units.json not found",
        fix: "Initialize fspec first: fspec init",
    },
];

const RELATED: &[&str] = &[
    "add-domain-event",
    "add-command",
    "add-policy",
    "show-event-storm",
    "generate-example-mapping-from-event-storm",
];

const NOTES: &[&str] = &[
    "External systems assigned stable IDs starting from 0",
    "Deleted external systems marked with deleted: true (soft delete)",
    "Use show-event-storm to view all Event Storm artifacts",
    "External systems help identify where to place adapters/ports in clean architecture",
    "Color: pink (matching physical Event Storming convention)",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-external-system",
    description:
        "Add external system to Event Storm section of work unit to identify third-party integrations and boundaries",
    usage: Some("fspec add-external-system <workUnitId> <text> [options]"),
    arguments: ARGS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use during Event Storming when identifying external systems, third-party services, databases, message queues, or file systems that your domain interacts with.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
