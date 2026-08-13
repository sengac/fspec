//! `list-scenario-tags` help configuration — Rust port of
//! `src/commands/list-scenario-tags-help.ts`.
//!
//! The output of `format_command_help(&CONFIG)` is byte-for-byte identical
//! to `node dist/index.js list-scenario-tags --help` when piped to non-TTY.
//!
//! Captured fixture: `rust/fspec/tests/fixtures/help/list-scenario-tags.txt`
//!
//! Note: the TS config is minimal — no whenToUse, no typicalWorkflow,
//! no commonErrors, no notes. The formatter omits those sections.

use super::super::{CommandArgument, CommandExample, CommandHelpConfig, CommandOption};

const EXAMPLE_1_OUTPUT: &str = "@edge-case\n@smoke";

const EXAMPLE_2_OUTPUT: &str = "@WORK-UNIT-001 (Work Unit Tags)\n@smoke (Test Type Tags)";

const EXAMPLES: &[CommandExample] = &[
    CommandExample {
        command:
            "fspec list-scenario-tags spec/features/login.feature \"Login with invalid password\"",
        description: Some("List scenario tags"),
        output: Some(EXAMPLE_1_OUTPUT),
    },
    CommandExample {
        command:
            "fspec list-scenario-tags spec/features/login.feature \"Valid user login\" --show-categories",
        description: Some("List tags with categories"),
        output: Some(EXAMPLE_2_OUTPUT),
    },
];

const ARGUMENTS: &[CommandArgument] = &[
    CommandArgument {
        name: "file",
        description: "Feature file path",
        required: true,
    },
    CommandArgument {
        name: "scenario",
        description: "Scenario name",
        required: true,
    },
];

const OPTIONS: &[CommandOption] = &[CommandOption {
    flag: "--show-categories",
    description: "Show tag categories from registry (spec/tags.json)",
    default_value: None,
}];

const RELATED: &[&str] = &["add-tag-to-scenario", "list-feature-tags"];

pub const CONFIG: CommandHelpConfig = CommandHelpConfig {
    name: "list-scenario-tags",
    description: "List all tags on a specific scenario",
    usage: Some("fspec list-scenario-tags <file> <scenario> [options]"),
    arguments: ARGUMENTS,
    options: OPTIONS,
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
