//! `add-command` help configuration — Rust port of
//! `src/commands/add-command-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js add-command --help`
//! (captured at `codelet/fspec/tests/fixtures/help/add-command.txt`).

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
            "Command text/name (PascalCase recommended, e.g., \"RegisterUser\", \"PlaceOrder\")",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--actor <actor>",
        description: "Actor who executes the command (e.g., \"User\", \"Admin\", \"System\")",
        default_value: None,
    },
    CommandOption {
        flag: "--timestamp <ms>",
        description: "Timeline timestamp in milliseconds (for temporal ordering)",
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
        command: "fspec add-command AUTH-001 \"RegisterUser\"",
        description: Some("Add command to work unit"),
        output: Some("✓ Added command \"RegisterUser\" to AUTH-001 (ID: 0)"),
    },
    CommandExample {
        command: "fspec add-command AUTH-001 \"LoginUser\" --actor \"User\"",
        description: Some("Add command with actor specification"),
        output: Some("✓ Added command \"LoginUser\" to AUTH-001 (ID: 1)"),
    },
    CommandExample {
        command:
            "fspec add-command CHECKOUT-001 \"PlaceOrder\" --actor \"Customer\" --bounded-context \"Sales\"",
        description: Some("Add command with actor and bounded context"),
        output: Some("✓ Added command \"PlaceOrder\" to CHECKOUT-001 (ID: 0)"),
    },
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

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet("Use PascalCase for command names (RegisterUser, PlaceOrder)"),
    CommonPatternEntry::Bullet("Commands represent user INTENTIONS (imperative verbs)"),
    CommonPatternEntry::Bullet("Commands typically trigger domain events (PlaceOrder → OrderPlaced)"),
    CommonPatternEntry::Bullet("Use --actor to specify WHO executes the command"),
    CommonPatternEntry::Bullet("Group related commands using --bounded-context flag"),
];

const RELATED: &[&str] = &[
    "add-domain-event",
    "add-policy",
    "add-hotspot",
    "show-event-storm",
    "show-foundation-event-storm",
    "generate-example-mapping-from-event-storm",
];

const NOTES: &[&str] = &[
    "Commands represent intentions that trigger state changes",
    "Commands assigned stable IDs starting from 0",
    "Deleted commands marked with deleted: true (soft delete)",
    "Use show-event-storm to view all Event Storm artifacts",
    "Bounded contexts help organize commands by subdomain (DDD pattern)",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-command",
    description: "Add command to Event Storm section of work unit for Big Picture Event Storming",
    usage: Some("fspec add-command <workUnitId> <text> [options]"),
    arguments: ARGUMENTS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use during Big Picture Event Storming discovery phase when capturing user intentions or system commands that trigger domain events.",
    ),
    when_not_to_use: None,
    prerequisites: &[
        "Work unit must exist",
        "Work unit must have eventStorm section initialized",
        "Command text should describe an ACTION (imperative verb)",
    ],
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
