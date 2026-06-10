//! `delete-epic` help configuration — Rust port of
//! `src/commands/delete-epic-help.ts`.
//!
//! Byte-for-byte parity with `node dist/index.js delete-epic --help`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGS: &[CommandArgument] = &[CommandArgument {
    name: "epicId",
    description: "Epic ID to delete",
    required: true,
}];

const OPTS: &[CommandOption] = &[CommandOption {
    flag: "--force",
    description: "Skip confirmation prompt",
    default_value: None,
}];

const EXAMPLE_OUTPUT: &str = "✓ Deleted epic old-feature";

const EXAMPLES: &[CommandExample] = &[CommandExample {
    command: "fspec delete-epic old-feature",
    description: Some("Delete epic with confirmation"),
    output: Some(EXAMPLE_OUTPUT),
}];

const RELATED: &[&str] = &["create-epic", "list-epics"];

const NOTES: &[&str] = &[
    "Work units associated with the epic are NOT deleted",
    "Epic associations are removed from work units",
];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "delete-epic",
    description: "Delete an epic (does not delete associated work units)",
    usage: Some("fspec delete-epic <epicId> [options]"),
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
