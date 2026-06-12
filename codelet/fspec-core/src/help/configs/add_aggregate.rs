//! `add-aggregate` help configuration — Rust port of
//! `src/commands/add-aggregate-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js add-aggregate --help`
//! (captured at `codelet/fspec/tests/fixtures/help/add-aggregate.txt`).

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
            "Aggregate name (PascalCase recommended, e.g., \"Order\", \"ShoppingCart\", \"UserAccount\")",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "--responsibilities <list>",
        description:
            "Comma-separated list of aggregate responsibilities (e.g., \"Validate order, Calculate total\")",
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
        command: "fspec add-aggregate CHECKOUT-001 \"Order\"",
        description: Some("Add aggregate to work unit"),
        output: Some("✓ Added aggregate \"Order\" to CHECKOUT-001 (ID: 0)"),
    },
    CommandExample {
        command:
            "fspec add-aggregate CHECKOUT-001 \"ShoppingCart\" --responsibilities \"Add items, Calculate total, Apply discounts\"",
        description: Some("Add aggregate with responsibilities"),
        output: Some("✓ Added aggregate \"ShoppingCart\" to CHECKOUT-001 (ID: 1)"),
    },
    CommandExample {
        command: "fspec add-aggregate AUTH-001 \"UserAccount\" --bounded-context \"Identity\"",
        description: Some("Add aggregate with bounded context"),
        output: Some("✓ Added aggregate \"UserAccount\" to AUTH-001 (ID: 0)"),
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

const COMMON_PATTERNS: &[CommonPatternEntry] = &[
    CommonPatternEntry::Bullet("Use PascalCase for aggregate names (Order, ShoppingCart, UserAccount)"),
    CommonPatternEntry::Bullet("Aggregates are nouns representing domain entities (not actions)"),
    CommonPatternEntry::Bullet("Group related aggregates using --bounded-context flag"),
    CommonPatternEntry::Bullet("Define responsibilities to clarify aggregate behavior"),
    CommonPatternEntry::Bullet("Aggregates enforce business invariants and consistency boundaries"),
];

const RELATED: &[&str] = &[
    "add-domain-event",
    "add-command",
    "add-bounded-context",
    "show-event-storm",
    "generate-example-mapping-from-event-storm",
];

const NOTES: &[&str] = &[
    "Aggregates are DDD pattern representing consistency boundaries",
    "Aggregates assigned stable IDs starting from 0",
    "Deleted aggregates marked with deleted: true (soft delete)",
    "Use show-event-storm to view all Event Storm artifacts",
    "Aggregates have yellow sticky notes in physical Event Storming",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "add-aggregate",
    description:
        "Add aggregate to Event Storm section of work unit for Process Modeling Event Storming",
    usage: Some("fspec add-aggregate <workUnitId> <text> [options]"),
    arguments: ARGUMENTS,
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use during Process Modeling Event Storming when identifying core domain entities (Aggregates) that enforce business rules and maintain consistency boundaries.",
    ),
    when_not_to_use: None,
    prerequisites: &[
        "Work unit must exist",
        "Work unit must have eventStorm section initialized",
        "Aggregate text should be a noun representing a domain entity (e.g., Order, User, Account)",
    ],
    common_patterns: COMMON_PATTERNS,
    typical_workflow: None,
    common_errors: COMMON_ERRORS,
    notes: NOTES,
};
