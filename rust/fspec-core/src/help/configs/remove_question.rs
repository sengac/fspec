//! `remove-question` help configuration — Rust port of the TS Commander.js
//! help output captured via `node dist/index.js remove-question --help`.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const ARGS: &[CommandArgument] = &[
    CommandArgument {
        name: "workUnitId",
        description: "Work unit ID",
        required: true,
    },
    CommandArgument {
        name: "index",
        description: "Question index (from show-work-unit)",
        required: true,
    },
];

const OPTS: &[CommandOption] = &[];

const EXAMPLES: &[CommandExample] = &[CommandExample {
    description: Some("Remove question at index 0"),
    command: "fspec remove-question AUTH-001 0",
    output: Some("✓ Removed question from AUTH-001"),
}];

const RELATED: &[&str] = &["add-question", "answer-question", "show-work-unit"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "remove-question",
    description: "Remove a question from Example Mapping by index",
    usage: Some("fspec remove-question <workUnitId> <index>"),
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
    notes: &[],
};
