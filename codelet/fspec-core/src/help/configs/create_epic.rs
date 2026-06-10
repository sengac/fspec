//! `create-epic` help configuration — Rust port of
//! `src/commands/create-epic-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js create-epic --help`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "epicId",
        description: "Epic ID in lowercase-with-hyphens format (e.g., user-management)",
        required: true,
    },
    CommandArgument {
        name: "title",
        description: "Human-readable title for the epic",
        required: true,
    },
];

const OPTS: &[CommandOption] = &[CommandOption {
    flag: "-d, --description <description>",
    description: "Optional description providing more context about the epic",
    default_value: None,
}];

const EXAMPLE_1_OUTPUT: &str = "✓ Created epic user-management\n  Title: User Management Features";
const EXAMPLE_2_OUTPUT: &str =
    "✓ Created epic auth\n  Title: Authentication System\n  Description: Handles user login and sessions";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command: "fspec create-epic user-management \"User Management Features\"",
        description: Some("Create epic with ID and title"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command:
            "fspec create-epic auth \"Authentication System\" -d \"Handles user login and sessions\"",
        description: Some("Create epic with description"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
];

const RELATED: &[&str] = &[
    "list-epics",
    "show-epic",
    "create-story",
    "create-bug",
    "create-task",
];

const NOTES: &[&str] = &[
    "Epic IDs must be lowercase with hyphens (no spaces or special characters)",
    "Epics help organize related work units into business initiatives",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "create-epic",
    description: "Create a new epic for organizing work units into high-level initiatives",
    usage: Some("fspec create-epic <epicId> <title>"),
    arguments: ARGS,
    options: OPTS,
    examples: EXAMPLES,
    related_commands: RELATED,
    when_to_use: None,
    when_not_to_use: None,
    prerequisites: &[],
    common_patterns: &[],
    typical_workflow: None,
    common_errors: &[],
    notes: NOTES,
};
