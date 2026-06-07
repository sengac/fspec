//! `list-work-units` help configuration — Rust port of
//! `src/commands/list-work-units-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js list-work-units --help`.

use super::super::{CommandExample, CommandHelpConfig, CommandOption};

const EXAMPLE_1_OUTPUT: &str = "AUTH-001 - User login feature\nUI-002 - Dashboard layout";

const EXAMPLE_2_OUTPUT: &str = "AUTH-003 - Password reset\nAPI-004 - User endpoints";

const EXAMPLE_3_OUTPUT: &str = "AUTH-001 - User login feature\nAUTH-003 - Password reset";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec list-work-units",
        description: Some("List all work units"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command: "fspec list-work-units --status=backlog",
        description: Some("List only backlog items"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
    CommandExample {
        command: "fspec list-work-units --prefix=AUTH",
        description: Some("List all AUTH-prefixed work units"),
        output: Some(EXAMPLE_3_OUTPUT),
    },
];

const OPTIONS: &[CommandOption] = &[
    CommandOption {
        flag: "-s, --status <status>",
        description:
            "Filter by workflow status: backlog, specifying, testing, implementing, validating, done, blocked",
        default_value: None,
    },
    CommandOption {
        flag: "--prefix <prefix>",
        description: "Filter by work unit prefix (e.g., AUTH, UI, API)",
        default_value: None,
    },
    CommandOption {
        flag: "--epic <epic>",
        description: "Filter by epic name",
        default_value: None,
    },
];

const RELATED: &[&str] = &[
    "show-work-unit",
    "create-story",
    "create-bug",
    "create-task",
    "update-work-unit-status",
    "board",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "list-work-units",
    description: "List work units with optional filtering by status, prefix, or epic",
    usage: Some("fspec list-work-units [options]"),
    arguments: &[],
    options: OPTIONS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: Some(
        "Use this command when you need to view work units in the backlog, see what's in progress, or filter by specific criteria like status, prefix, or epic.",
    ),
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: Some(
        "backlog → specifying → testing → implementing → validating → done. Use --status flag to see work units at each stage.",
    ),
    common_errors: &[],
    notes: &[],
};
