//! `add-policy` help configuration — Rust port of
//! `src/commands/add-policy-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js add-policy --help`.

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
        description: "Policy text (brief description)",
        required: true,
    },
];

const OPTS: &[CommandOption] = &[
    CommandOption {
        flag: "--when <event>",
        description: "Event that triggers the policy (e.g., \"UserRegistered\")",
        default_value: None,
    },
    CommandOption {
        flag: "--then <command>",
        description: "Command that executes when policy triggers (e.g., \"SendWelcomeEmail\")",
        default_value: None,
    },
    CommandOption {
        flag: "--timestamp <ms>",
        description: "Timeline position in milliseconds",
        default_value: None,
    },
    CommandOption {
        flag: "--bounded-context <name>",
        description: "Bounded context association",
        default_value: None,
    },
];

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command:
            "fspec add-policy AUTH-001 \"Send welcome email\" --when \"UserRegistered\" --then \"SendWelcomeEmail\"",
        description: Some("Add policy with event/command relationship"),
        output: Some("✓ Added policy \"Send welcome email\" to AUTH-001 (ID: 0)"),
    },
    CommandExample {
        command:
            "fspec add-policy ORDER-001 \"Notify warehouse\" --when \"OrderPlaced\" --then \"NotifyWarehouse\"",
        description: Some("Add reactive policy for order processing"),
        output: Some("✓ Added policy \"Notify warehouse\" to ORDER-001 (ID: 0)"),
    },
];

const RELATED: &[&str] = &[
    "add-domain-event",
    "add-command",
    "add-hotspot",
    "show-event-storm",
    "generate-example-mapping-from-event-storm",
];

const PREREQUISITES: &[&str] = &[
    "Work unit must exist",
    "Work unit must have eventStorm section initialized",
    "Policy should describe WHEN/THEN relationship",
];

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet("Policies represent REACTIVE business logic (automation)"),
    CommonPatternEntry::Bullet("Use WHEN/THEN pattern: WHEN event THEN command"),
    CommonPatternEntry::Bullet(
        "Policies convert to business rules via generate-example-mapping-from-event-storm",
    ),
    CommonPatternEntry::Bullet("Policies often represent system behaviors (not user actions)"),
];

const COMMON_ERRORS: &[CommonError] = &[CommonError {
    error: "Work unit not found",
    fix: "Ensure work unit exists: fspec show-work-unit <id>",
}];

const NOTES: &[&str] = &[
    "Policies are automation rules triggered by events",
    "Format: \"WHEN event happens THEN execute command\"",
    "Policies assigned stable IDs starting from 0",
    "Use show-event-storm to view all policies",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-policy",
    description:
        "Add policy to Event Storm section for reactive business logic (WHEN event THEN command)",
    usage: Some("fspec add-policy <workUnitId> <text> [options]"),
    arguments: ARGS,
    options: OPTS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use during Big Picture Event Storming when capturing reactive business logic that triggers automatically when events occur.",
    ),
    when_not_to_use: None,
    prerequisites: PREREQUISITES,
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
